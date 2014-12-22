use std::io::IoResult;
pub use self::TelnetEvent::*;
pub use self::TelnetCommand::*;
pub use self::TelnetOption::*;

pub const ECHO: u8 =   1;
pub const WILL: u8 = 251;
pub const WONT: u8 = 252;
pub const DO:   u8 = 253;
pub const DONT: u8 = 254;
pub const IAC:  u8 = 255;

mod state {
    use super::{TelnetCommand, TelnetOption};
    pub use self::State::*;

    #[deriving(Copy)]
    pub enum State {
        Data,
        Iac,
        Negotiating(fn(TelnetOption) -> TelnetCommand)
    }

    impl PartialEq for State {
        fn eq(&self, other: &State) -> bool {
            use self::State::*;

            match (*self, *other) {
                (Data, Data) | (Iac, Iac) => true,
                // vv this is gross, but it works for its purpose I think.
                // (just don't ever expect Negotiating(f) != Negotiating(g))
                (Negotiating(_), Negotiating(_)) => true,
                _ => false
            }
        }
    }
}

#[deriving(PartialEq, Eq, Show)]
pub enum TelnetEvent {
    Data(Vec<u8>),
    Command(TelnetCommand)
}

#[deriving(PartialEq, Eq, Show)]
pub enum TelnetCommand {
    Will(TelnetOption),
    Wont(TelnetOption),
    Do(TelnetOption),
    Dont(TelnetOption),
    UnknownCommand(u8)
}

#[deriving(PartialEq, Eq, Show)]
pub enum TelnetOption {
    Echo,
    UnknownOption(u8)
}

impl TelnetOption {
    fn from_u8(x: u8) -> TelnetOption {
        use self::TelnetOption::*;

        match x {
            ECHO => Echo,
            _ => UnknownOption(x)
        }
    }
}

pub struct Telnet<R> {
    rdr: R,
    buf: [u8, ..1024],
    state: state::State
}

impl<R> Telnet<R> where R: Reader {
    pub fn new(reader: R) -> Telnet<R> {
        Telnet {
            rdr: reader,
            buf: [0, ..1024],
            state: state::Data
        }
    }

    /// `FIXME`: should this take a callback function instead? Would avoid possibly
    /// spurious allocations, especially in the common case of only generating a single
    /// `Data` event. Or this could use something like SmallVector from the syntax crate.
    /// (/me wishes for `yield` and impl Iterator)
    pub fn read_events(&mut self) -> IoResult<Vec<TelnetEvent>> {
        let mut res = vec![];
        let nbytes = try!(self.rdr.read(&mut self.buf));
        let buf = self.buf.slice_to(nbytes);

        let mut from = 0; // Marks the boundary of the last event in the buffer
        for (i, &x) in buf.iter().enumerate() {
            match self.state {
                state::Data => {
                    if x == IAC {
                        // Wrap up + ship off any data preceding this byte
                        if from < i {
                            let data = buf.slice(from, i).to_vec();
                            res.push(Data(data));
                        }
                        self.state = state::Iac;
                        from = i + 1;
                    }
                }
                state::Iac => {
                    self.state = match x {
                        IAC => { // IAC escaping
                            res.push(Data(vec![x]));
                            from = i + 1;
                            state::Data
                        }
                        // These commands expect another byte to specify the option
                        WILL => state::Negotiating(Will),
                        WONT => state::Negotiating(Wont),
                        DO   => state::Negotiating(Do),
                        DONT => state::Negotiating(Dont),
                        _ => {
                            // Other commands don't expect another byte, so we're done
                            res.push(Command(UnknownCommand(x)));
                            state::Data
                        }
                    };
                }
                state::Negotiating(cmd) => {
                    let opt = TelnetOption::from_u8(x);
                    res.push(Command(cmd(opt)));
                    self.state = state::Data;
                    from = i + 1;
                }
            }
        }
        // Push any data left over from the buffer as a Data event
        if from < buf.len() {
            let data = buf.slice_from(from).to_vec();
            res.push(Data(data));
        }
        Ok(res)
    }
}

#[cfg(test)]
mod test {
    use super::{Telnet, Command, Data, Will, Echo};
    use super::{IAC, WILL, ECHO};
    use std::io::{BufReader, MemReader};

    #[test]
    fn test_empty() {
        let buf = [];
        let mut telnet = Telnet::new(BufReader::new(&buf));
        let evts = telnet.read_events();
        assert!(evts.is_err());
    }

    #[test]
    fn test_data() {
        let buf = b"Hello, world!".to_vec();
        let mut telnet = Telnet::new(MemReader::new(buf.clone()));
        let evts = telnet.read_events().unwrap();
        assert_eq!(evts[0], Data(buf));
    }

    #[test]
    fn test_iac() {
        let buf = vec![IAC, WILL, ECHO];
        let mut telnet = Telnet::new(MemReader::new(buf));
        let evts = telnet.read_events().unwrap();
        assert_eq!(evts[0], Command(Will(Echo)))
    }

    #[test]
    fn test_mixed_iac_data() {
        let data1 = b"Hello, world!";
        let cmd = [IAC, WILL, ECHO];
        let data2 = b"Goodbye, world!";
        let mut all = data1.to_vec();
        all.push_all(&cmd);
        all.push_all(data2);
        let mut telnet = Telnet::new(MemReader::new(all));
        let evts = telnet.read_events().unwrap();
        assert_eq!(evts.len(), 3);
        assert_eq!(evts[0], Data(data1.to_vec()));
        assert_eq!(evts[1], Command(Will(Echo)));
        assert_eq!(evts[2], Data(data2.to_vec()));
    }

    #[test]
    fn test_iac_escaping() {
        let data = [IAC, IAC];
        let mut telnet = Telnet::new(BufReader::new(&data));
        let evts = telnet.read_events().unwrap();
        println!("{}", evts);
        assert_eq!(evts.len(), 1);
        assert_eq!(evts[0], Data(vec![IAC]));
    }
}
