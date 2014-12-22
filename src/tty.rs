use std::io::IoResult;

pub use self::Toggle::*;

use posix::termios::{mod, ECHO, TCSANOW};
use posix::unistd::STDIN_FILENO;

macro_rules! try_errno {
    ($e:expr) => {
        {
            use std::io::IoError;
            use std::os::errno;
            let err = $e;
            if err != 0 {
                return Err(IoError::from_errno(errno() as uint, true));
            }
        }
    }
}

pub enum Toggle {
    On,
    Off
}

/// Note: `Tty` stores the current termios state in the constructor and then restores it
/// in the dtor.
#[deriving(Copy)]
pub struct Tty {
    orig: termios::termios
}

impl Tty {
    pub fn new() -> IoResult<Tty> {
        Ok(Tty {
            orig: try!(read_termios())
        })
    }

    pub fn echo(&mut self, enabled: Toggle) -> IoResult<()> {
        let mut tp = try!(read_termios());
        match enabled {
            On => tp.c_lflag |= ECHO,
            Off => tp.c_lflag &= !ECHO
        }
        set_termios(tp)
    }
}

impl Drop for Tty {
    fn drop(&mut self) {
        let _ = set_termios(self.orig);
    }
}

fn read_termios() -> IoResult<termios::termios> {
    let mut tp = termios::termios::new();
    try_errno!(termios::tcgetattr(STDIN_FILENO, &mut tp));
    Ok(tp)
}

fn set_termios(mut tp: termios::termios) -> IoResult<()> {
    try_errno!(termios::tcsetattr(STDIN_FILENO, TCSANOW, &mut tp));
    Ok(())
}
