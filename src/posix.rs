// These are wrong yeah yeah

#![allow(non_camel_case_types)]

pub mod termios {
    use libc::{c_int, c_uchar, c_uint};

    pub const ECHO: tcflag_t = 0o000010;
    pub const TCSANOW: c_int = 0;

    pub type cc_t = c_uchar;
    pub type speed_t = c_uint;
    pub type tcflag_t = c_uint;

    #[repr(C)]
    #[deriving(Copy)]
    pub struct termios {
        pub c_iflag: tcflag_t,
        pub c_oflag: tcflag_t,
        pub c_cflag: tcflag_t,
        pub c_lflag: tcflag_t,
        pub c_line: cc_t,
        pub c_cc: [cc_t, ..32u],
        pub c_ispeed: speed_t,
        pub c_ospeed: speed_t,
    }

    impl termios {
        pub fn new() -> termios {
            unsafe { ::std::mem::zeroed() }
        }
    }

    pub fn tcgetattr(fd: c_int, termios_p: &mut termios) -> c_int {
        extern { fn tcgetattr(fd: c_int, termios_p: *mut termios) -> c_int; }
        unsafe { tcgetattr(fd, termios_p as *mut _) }
    }

    pub fn tcsetattr(fd: c_int, optional_actions: c_int, termios_p: &termios) -> c_int {
        extern { fn tcsetattr(fd: c_int, optional_actions: c_int,
                              termios_p: *const termios) -> c_int; }
        unsafe { tcsetattr(fd, optional_actions, termios_p as *const _) }
    }
}

pub mod unistd {
    pub const STDIN_FILENO: ::libc::c_int = 0;
}
