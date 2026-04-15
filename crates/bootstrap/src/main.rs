#![no_std]
#![no_main]

use syscall::Rights;

#[cfg_attr(not(test), panic_handler)]
#[allow(unused)]
fn panic(info: &core::panic::PanicInfo) -> ! {
    syscall::log("panic");
    loop {
        hal::halt();
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    let echo_proc_id = syscall::spawn(echo, 0);

    let send_ep_id = syscall::create_endpoint().expect("could not create endpoint");
    let recv_ep_id = syscall::derive_cap(send_ep_id, Rights::RWE);
    let echo_proc_ep_id = syscall::move_cap(echo_proc_id, recv_ep_id);
    assert!(echo_proc_ep_id == 0);

    loop {
        syscall::log("send message to echo");
        let msg = "send to echo".into();
        syscall::send(send_ep_id,msg).expect("could not send");
        syscall::switch(echo_proc_id);
        let resp = syscall::recv(send_ep_id).expect("could not receive");
        assert_eq!(msg, resp);
        syscall::log("received message from echo");
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn echo(cap_id: usize) -> usize {
    loop {
        syscall::log("receive message from bs");
        let msg = syscall::recv(cap_id).expect("could not recv");
        syscall::send(cap_id, msg).expect("could not send");
        syscall::log("send message back to bs");
        syscall::switch(1);
    }
}

use core::cmp::min;
use core::fmt;
use core::str::from_utf8;

/// A struct representing a writer that appends formatted data to a byte buffer.
pub struct WriteTo<'a> {
    buf: &'a mut [u8],
    len: usize,
}

impl<'a> WriteTo<'a> {
    /// Constructs a new `WriteTo` instance wrapping the provided byte buffer.
    pub fn new(buf: &'a mut [u8]) -> Self {
        WriteTo { buf, len: 0 }
    }

    /// Converts the written portion of the buffer into a string slice, if possible.
    pub fn as_str(self) -> Option<&'a str> {
        if self.len <= self.buf.len() {
            from_utf8(&self.buf[..self.len]).ok()
        } else {
            None
        }
    }

    /// Get the number of bytes written to buffer, unless there where errors.
    pub fn len(&self) -> Option<usize> {
        if self.len <= self.buf.len() {
            Some(self.len)
        } else {
            None
        }
    }

    /// Returns true if self has a length of zero bytes, unless there where errors.
    pub fn is_empty(&self) -> Option<bool> {
        if self.len <= self.buf.len() {
            Some(self.len == 0)
        } else {
            None
        }
    }
}

impl<'a> fmt::Write for WriteTo<'a> {
    /// Writes a string slice into the buffer, updating the length accordingly.
    fn write_str(&mut self, s: &str) -> fmt::Result {
        if self.len > self.buf.len() {
            return Err(fmt::Error);
        }

        let rem = &mut self.buf[self.len..];
        let raw_s = s.as_bytes();
        let num = min(raw_s.len(), rem.len());

        rem[..num].copy_from_slice(&raw_s[..num]);
        self.len += raw_s.len();

        if num < raw_s.len() {
            Err(fmt::Error)
        } else {
            Ok(())
        }
    }
}

/// Formats data using `format_args!` (`arg` argument) and writes it to a byte buffer `buf`.
pub fn show<'a>(buf: &'a mut [u8], arg: fmt::Arguments) -> Result<&'a str, fmt::Error> {
    let mut w = WriteTo::new(buf);
    fmt::write(&mut w, arg)?;
    w.as_str().ok_or(fmt::Error)
}
