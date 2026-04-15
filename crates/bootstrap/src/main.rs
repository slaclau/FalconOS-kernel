#![no_std]
#![no_main]

use syscall::cap::{CapHandle, Rights};
use syscall::ipc::Endpoint;
use syscall::process::Process;

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
    let args = EchoArgs {
        parent_handle: 0,
        ep_handle: 1
    };
    let echo_proc = syscall::cap::Cap::<syscall::process::Process>::spawn(echo, &args as *const _ as usize);

    let send_ep = syscall::ipc::create_endpoint().expect("could not create endpoint");
    let recv_ep = send_ep.derive(Rights::RWE);
    recv_ep.r#move(echo_proc);

    loop {
        syscall::log("send message to echo");
        let msg = "send to echo".into();
        send_ep.send(msg).expect("could not send");
        echo_proc.switch();
        let resp = send_ep.recv().expect("could not receive");
        assert_eq!(msg, resp);
        syscall::log("received message from echo");
    }
}

#[derive(Clone, Copy)]
pub struct EchoArgs {
    parent_handle: CapHandle,
    ep_handle: CapHandle,
}

#[unsafe(no_mangle)]
pub extern "C" fn echo(arg_ptr: usize) -> usize {
    let args = unsafe{*(arg_ptr as *const EchoArgs)};
    let parent_proc = unsafe { syscall::cap::Cap::<Process>::from_handle(args.parent_handle) };
    let ep = unsafe { syscall::cap::Cap::<Endpoint>::from_handle(args.ep_handle) };
    loop {
        syscall::log("receive message from bs");
        let msg = ep.recv().expect("could not recv");
        ep.send(msg).expect("could not send");
        syscall::log("send message back to bs");
        parent_proc.switch();
    }
}

use core::cmp::min;
use core::{fmt, usize};
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
