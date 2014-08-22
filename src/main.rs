use std::comm;
use std::io::{BufferedReader, IoResult, LineBufferedWriter, TcpStream};

struct Conn {
    pub rx: comm::Receiver<Vec<u8>>,
    pub tx: comm::Sender<String>
}

impl Conn {
    fn new(host: &str, port: u16) -> IoResult<Conn> {
        let stream = try!(TcpStream::connect(host, port));
        let (server_tx, server_rx) = comm::channel();
        let server_stream = stream.clone();

        spawn(proc() {
            let mut reader = BufferedReader::new(server_stream);
            let mut buf = [0u8, ..1024];
            loop {
                let n = reader.read(buf).unwrap();
                if n == 0 {
                    continue;
                }
                let vec = Vec::from_slice(buf.slice_to(n));
                server_tx.send(vec);
            }
        });

        let (client_tx, client_rx) = comm::channel();
        let client_stream = stream.clone();

        spawn(proc() {
            let mut writer = LineBufferedWriter::new(client_stream);
            loop {
                let inp: String = client_rx.recv();
                writer.write_str(inp.as_slice());
            }
        });

        Ok(Conn {
            rx: server_rx,
            tx: client_tx
        })
    }
}

fn main() {
    use std::ascii::AsciiCast;

    let stdin = std::io::stdio::stdin();
    let (inp_tx, inp_rx) = comm::channel();
    spawn(proc() {
        let mut stdin = stdin;
        for line in stdin.lines() {
            inp_tx.send(line.unwrap());
        }
    });

    let conn = Conn::new("localhost", 2424).unwrap_or_else(|e| {
        fail!("connection error: {}", e)
    });
    let (conn_tx, conn_rx) = (conn.tx, conn.rx);

    loop {
        select! {
            xs = conn_rx.recv() => {
                for x in xs.iter() {
                    if x.is_ascii() {
                        print!("{}", x.to_ascii());
                    }
                }
                std::io::stdio::flush();
            },
            inp = inp_rx.recv() => conn_tx.send(inp)
        }
    }
}
