#![feature(globs, macro_rules, phase)]

#[phase(plugin, link)] extern crate log;
extern crate posix;

use std::comm;
use std::io::{BufferedReader, IoResult, TcpStream};

use cmd::Command;
use telnet::TelnetEvent;

mod telnet;
mod tty;

enum ParsedInput {
    CommandInput(Command),
    RegularInput(String)
}

mod cmd {
    /// Built in commands and 'other' commands to be passed off to extensions
    #[deriving(Show)]
    pub enum Command {
        Quit,
        Other(String)
    }

    impl Command {
        pub fn from_string(name: String) -> Command {
            match name.as_slice() {
                "quit" => Quit,
                _ => Other(name)
            }
        }
    }
}

type ConnReadTx = comm::Sender<TelnetEvent>;
type ConnWriteRx = comm::Receiver<Vec<u8>>;

struct Conn {
    stream: TcpStream
}

impl Conn {
    fn new(host: &str, port: u16, read_tx: ConnReadTx,
           write_rx: ConnWriteRx) -> IoResult<Conn> {
        let stream = try!(TcpStream::connect(host, port));
        let server_stream = stream.clone();

        spawn(proc() {
            use telnet::Telnet;
            let reader = BufferedReader::new(server_stream);
            let mut telnet = Telnet::new(reader);

            loop {
                let evts = telnet.read_events();
                let evts = match evts {
                    Ok(evts) => evts,
                    Err(..) => break
                };
                for evt in evts.move_iter() {
                    read_tx.send(evt);
                }
            }
        });

        let client_stream = stream.clone();

        spawn(proc() {
            let mut writer = client_stream;
            loop {
                let inp: Result<Vec<u8>, ()> = write_rx.recv_opt();
                let write = match inp {
                    Ok(ref inp) => writer.write(inp.as_slice()),
                    Err(..) => break
                };
                match write {
                    Ok(..) => {},
                    Err(..) => break
                }
            }
        });

        Ok(Conn { stream: stream })
    }

    fn close(&mut self) -> IoResult<()> {
        try!(self.stream.close_read());
        try!(self.stream.close_write());
        Ok(())
    }
}

fn extract_args() -> Result<(String, u16), String> {
    use std::from_str::FromStr;

    let usage = "Usage: quagmire <host> <port>".to_string();
    let (host, port) = match std::os::args().as_slice() {
        [_, ref host, ref port] => {
            (host.clone(), port.clone())
        },
        _ => return Err(usage)
    };
    let port: u16 = match FromStr::from_str(port.as_slice()) {
        Some(p) => p,
        None => return Err(usage)
    };
    Ok((host, port))
}

fn parse_input(inp: String) -> ParsedInput {
    {
        let mut toks = inp.as_slice().trim().split(' ');
        let first = toks.next().unwrap(); // this is safe
        if first.starts_with("/") && !first.starts_with("//") {
            let cmd = Command::from_string(first.slice_from(1).to_string());
            return CommandInput(cmd);
        }
    }
    RegularInput(inp)
}

pub fn main() {
    use std::ascii::AsciiCast;
    use telnet as tel;

    let mut tty = tty::Tty::new();

    debug!("is stdin a tty? {}", tty.is_ok());

    let mut stderr = std::io::stdio::stderr();
    let (host, port) = match extract_args() {
        Ok(r) => r,
        Err(e) => {
            (writeln!(stderr, "{}", e)).unwrap();
            std::os::set_exit_status(64);
            return;
        }
    };

    let stdin = std::io::stdio::stdin();
    let (inp_tx, inp_rx) = comm::channel();

    spawn(proc() {
        let mut stdin = stdin;
        for line in stdin.lines() {
            let line = line.unwrap_or_else(|e| fail!("Couldn't read line: {}", e));
            let inp = parse_input(line);
            // Ugh, feels like an awful hack. And it won't work for quits induced in some
            // other way (like, the server closing the connection).
            let done = match inp {
                CommandInput(cmd::Quit) => true,
                _ => false
            };
            if done { return }
            inp_tx.send(inp);
        }
    });

    let (conn_write_tx, conn_write_rx) = comm::channel();
    let (conn_read_tx, conn_read_rx) = comm::channel();
    let mut conn = Conn::new(host.as_slice(), port, conn_read_tx,
                             conn_write_rx).unwrap_or_else(|e| {
        fail!("connection error: {}", e)
    });
    let (raw_inp_tx, raw_inp_rx) = comm::channel();

    'main: loop {
        select! {
            event = conn_read_rx.recv_opt() => {
                let event = match event {
                    Ok(evt) => evt,
                    Err(e) => {
                        error!("{}", e);
                        break 'main;
                    }
                };
                match event {
                    telnet::Data(xs) => {
                        for x in xs.iter() {
                            if x.is_ascii() {
                                print!("{}", x.to_ascii());
                            }
                        }
                        std::io::stdio::flush();
                    }
                    tel::Command(tel::Will(tel::Echo)) => {
                        debug!("received WILL ECHO");
                        raw_inp_tx.send(vec![tel::IAC, tel::DO, tel::ECHO]);
                        match tty.as_mut().map(|t| t.echo(tty::Off)) {
                            Ok(_) => {},
                            Err(e) => error!("Couldn't disable echo: {}", e)
                        }
                    }
                    tel::Command(tel::Wont(tel::Echo)) => {
                        debug!("received WONT ECHO");
                        raw_inp_tx.send(vec![tel::IAC, tel::DONT, tel::ECHO]);
                        match tty.as_mut().map(|t| t.echo(tty::On)) {
                            Ok(_) => {},
                            Err(e) => error!("Couldn't disable echo: {}", e)
                        }
                    }
                    cmd @ tel::Command(_) => {
                        debug!("Got command {}", cmd);
                    }
                }
            },
            inp = inp_rx.recv_opt() => {
                let inp = match inp {
                    Err(_) => {
                        debug!("Received EOF, exiting.");
                        break 'main
                    },
                    Ok(inp) => inp
                };
                match inp {
                    CommandInput(cmd) => {
                        match cmd {
                            cmd::Quit => {
                                break 'main
                            }
                            cmd::Other(_) => println!("DO COMMAND: {}", cmd)
                        }
                    },
                    RegularInput(s) => conn_write_tx.send(s.into_bytes())
                }
            },
            raw_inp = raw_inp_rx.recv() => conn_write_tx.send(raw_inp)
        }
    }
    match conn.close() {
        Ok(..) => {},
        Err(e) => error!("Couldn't close connection: {}", e)
    }
}
