#![feature(globs, macro_rules, phase)]

#[phase(plugin, link)] extern crate log;
extern crate posix;

use std::comm;
use std::io::{BufferedReader, IoResult, TcpStream};

use telnet::TelnetEvent;

mod telnet;
mod tty;

struct Conn {
    pub rx: comm::Receiver<TelnetEvent>,
    pub tx: comm::Sender<Vec<u8>>
}

impl Conn {
    fn new(host: &str, port: u16) -> IoResult<Conn> {
        let stream = try!(TcpStream::connect(host, port));
        let (server_tx, server_rx) = comm::channel();
        let server_stream = stream.clone();

        spawn(proc() {
            use telnet::Telnet;
            let reader = BufferedReader::new(server_stream);
            let mut telnet = Telnet::new(reader);

            loop {
                let evts = telnet.read_events().unwrap();
                for evt in evts.move_iter() {
                    server_tx.send(evt);
                }
            }
        });

        let (client_tx, client_rx) = comm::channel();
        let client_stream = stream.clone();

        spawn(proc() {
            let mut writer = client_stream;
            loop {
                let inp: Vec<u8> = client_rx.recv();
                writer.write(inp.as_slice()).unwrap();
            }
        });

        Ok(Conn {
            rx: server_rx,
            tx: client_tx
        })
    }
}

pub fn main() {
    use std::ascii::AsciiCast;
    use telnet as tel;

    let mut tty = tty::Tty::new();

    let stdin = std::io::stdio::stdin();
    debug!("is stdin a tty? {}", tty.is_ok());

    let (inp_tx, inp_rx) = comm::channel();
    let inp_tx2 = inp_tx.clone();
    spawn(proc() {
        let mut stdin = stdin;
        for line in stdin.lines() {
            match line {
                Ok(line) => inp_tx.send(line.into_bytes()),
                Err(e) => error!("Couldn't read line: {}", e)
            }
        }
    });

    let conn = Conn::new("localhost", 2525).unwrap_or_else(|e| {
        fail!("connection error: {}", e)
    });
    let (conn_tx, conn_rx) = (conn.tx, conn.rx);

    loop {
        select! {
            event = conn_rx.recv() => {
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
                        inp_tx2.send(vec![tel::IAC, tel::DO, tel::ECHO]);
                        match tty.as_mut().map(|t| t.echo(tty::Off)) {
                            Ok(_) => {},
                            Err(e) => error!("Couldn't disable echo: {}", e)
                        }
                    }
                    tel::Command(tel::Wont(tel::Echo)) => {
                        debug!("received WONT ECHO");
                        inp_tx2.send(vec![tel::IAC, tel::DONT, tel::ECHO]);
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
            inp = inp_rx.recv() => conn_tx.send(inp)
        }
    }
}
