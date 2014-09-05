use std::comm;
use std::io::{BufferedReader, IoResult, TcpStream};
use super::telnet::TelnetEvent;

pub type ConnReadTx = comm::Sender<TelnetEvent>;
pub type ConnWriteRx = comm::Receiver<Vec<u8>>;

pub struct Conn {
    stream: TcpStream
}

impl Conn {
    pub fn new(host: &str, port: u16, read_tx: ConnReadTx,
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

    pub fn close(&mut self) -> IoResult<()> {
        try!(self.stream.close_read());
        try!(self.stream.close_write());
        Ok(())
    }
}

