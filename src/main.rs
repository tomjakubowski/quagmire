#![feature(phase, globs)]

#[phase(plugin, link)] extern crate log;

use std::comm;
use std::io::{BufferedReader, IoResult, LineBufferedWriter, TcpStream};

use telnet::TelnetEvent;

mod telnet;

struct Conn {
    pub rx: comm::Receiver<TelnetEvent>,
    pub tx: comm::Sender<String>
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
            let mut writer = LineBufferedWriter::new(client_stream);
            loop {
                let inp: String = client_rx.recv();
                writer.write_str(inp.as_slice()).unwrap();
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

    let stdin = std::io::stdio::stdin();
    let is_tty = stdin.get_ref().isatty();
    debug!("stdin is a tty? {}", is_tty);

    let (inp_tx, inp_rx) = comm::channel();
    spawn(proc() {
        let mut stdin = stdin;
        for line in stdin.lines() {
            inp_tx.send(line.unwrap());
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
                    }
                    tel::Command(tel::Wont(tel::Echo)) => {
                        debug!("received WONT ECHO");
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
