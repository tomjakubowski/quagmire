use std::comm;
use std::io::{BufferedReader, IoResult, LineBufferedWriter, TcpStream};

struct Conn {
    stream: TcpStream,
    pub rx: comm::Receiver<u8>,
    pub tx: comm::Sender<String>
}

impl Conn {
    fn new(host: &str, port: u16) -> IoResult<Conn> {
        let stream = try!(TcpStream::connect(host, port));
        let (server_tx, server_rx) = comm::channel();
        let server_stream = stream.clone();

        spawn(proc() {
            let mut reader = BufferedReader::new(server_stream);
            loop {
                let byte = reader.read_byte(); // lol
                server_tx.send(byte.unwrap());
            }
        });

        let (client_tx, client_rx) = comm::channel();
        let mut client_stream = stream.clone();

        spawn(proc() {
            let mut writer = LineBufferedWriter::new(client_stream);
            loop {
                let inp: String = client_rx.recv();
                writer.write_str(inp.as_slice());
            }
        });

        Ok(Conn {
            stream: stream,
            rx: server_rx,
            tx: client_tx
        })
    }
}

fn main() {
    use std::ascii::AsciiCast;
    use std::io::timer::Timer;
    use std::time::Duration;

    let conn = Conn::new("localhost", 2424).unwrap_or_else(|e| {
        fail!("connection error: {}", e)
    });

    let mut timer = Timer::new().unwrap();
    let flush = timer.periodic(Duration::milliseconds(100));

    let (conn_tx, conn_rx) = (conn.tx, conn.rx);

    let mut stdin = std::io::stdio::stdin();

    let (inp_tx, inp_rx) = comm::channel();

    spawn(proc() {
        for line in stdin.lines() {
            inp_tx.send(line.unwrap());
        }
    });

    loop {
        select! {
            x = conn_rx.recv() => {
                if x.is_ascii() { // FIXME
                    print!("{}", x.to_ascii())
                }
            },
            inp = inp_rx.recv() => conn_tx.send(inp),
            () = flush.recv() => std::io::stdio::flush()
        }
    }
}
