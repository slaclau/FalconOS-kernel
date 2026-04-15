#![no_std]
#![feature(const_trait_impl)]
#![feature(const_ops)]

mod arch;

use core::{
    fmt::Debug,
    mem,
    ops::{BitAnd, BitOr},
};

use arch::*;

pub const SYS_SWITCH: usize = 0;
pub const SYS_GET_PID: usize = 1;
pub const SYS_SPAWN: usize = 2;
pub const SYS_EXIT: usize = 3;
pub const SYS_WAIT: usize = 4;
pub const SYS_LOG: usize = 5;

pub const SYS_CREATE_ENDPOINT: usize = 6;
pub const SYS_RECV: usize = 7;
pub const SYS_SEND: usize = 8;

pub const SYS_DERIVE_CAP: usize = 9;
pub const SYS_MOVE_CAP: usize = 10;

pub type ProcessId = usize;

pub fn switch(process_id: ProcessId) -> ProcessId {
    unsafe { syscall1(SYS_SWITCH, process_id) }
}

pub fn get_pid() -> ProcessId {
    unsafe { syscall0(SYS_GET_PID) }
}

pub fn spawn(entry: extern "C" fn(arg: usize) -> usize, arg: usize) -> ProcessId {
    unsafe { syscall2(SYS_SPAWN, entry as usize, arg) }
}

pub fn exit(exit_code: usize) -> ! {
    unsafe { syscall1(SYS_EXIT, exit_code) };
    unreachable!()
}

pub fn wait(pid: ProcessId) -> usize {
    unsafe { syscall1(SYS_WAIT, pid) }
}

pub fn log(message: &str) -> usize {
    unsafe { syscall2(SYS_LOG, message.as_ptr() as usize, message.len()) }
}

#[derive(Clone, Copy)]
pub struct Rights(u8);

impl Rights {
    pub const ALL: Rights = Rights(u8::MAX);
    pub const READ: Rights = Rights(0x1);
    pub const WRITE: Rights = Rights(0x2);
    pub const EXEC: Rights = Rights(0x4);
    pub const GRANT: Rights = Rights(0x8);
    pub const RWE: Rights = Rights::READ | Rights::WRITE | Rights::EXEC;

    pub fn read(self) -> bool {
        (self & Self::READ).0 > 0
    }

    pub fn write(self) -> bool {
        (self & Self::WRITE).0 > 0
    }

    pub fn exec(self) -> bool {
        (self & Self::EXEC).0 > 0
    }

    pub fn grant(self) -> bool {
        (self & Self::GRANT).0 > 0
    }

    pub fn matches(self, mask: Rights) -> bool {
        (self & mask).0 > 0
    }
}

impl Debug for Rights {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Rights")
            .field("R", &self.read())
            .field("W", &self.write())
            .field("X", &self.exec())
            .field("G", &self.grant())
            .finish()
    }
}

impl const BitAnd for Rights {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl const BitOr for Rights {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl From<usize> for Rights {
    fn from(value: usize) -> Self {
        assert!(value <= u8::MAX as usize);
        Self(value as u8)
    }
}

pub fn create_endpoint() -> Result<usize, &'static str> {
    let res = unsafe { syscall0(SYS_CREATE_ENDPOINT) };
    Ok(res)
}

pub fn send(ep_id: usize, message: Message) -> Result<(), IpcError> {
    let code = unsafe {
        syscall5(
            SYS_SEND,
            ep_id,
            message.data[0],
            message.data[1],
            message.data[2],
            message.data[3],
        )
    };
    if code == 0 { Ok(()) } else { Err(code.into()) }
}

pub fn recv(ep_id: usize) -> Result<Message, IpcError> {
    let (res, words) = unsafe { out_syscall5(SYS_RECV, ep_id, 0, 0, 0, 0) };

    if res == 0 {
        Ok(Message {
            data: *words[1..5].as_array().unwrap(),
        })
    } else {
        Err(res.into())
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Message {
    pub data: [usize; 4],
}

impl Message {
    pub fn to_string(self, bytes_buf: &mut [u8; 32]) -> &str {
        let bytes = self.data.iter().flat_map(|word| word.to_be_bytes());

        for (i, byte) in bytes.enumerate() {
            bytes_buf[i] = byte;
        }
        str::from_utf8(bytes_buf).unwrap()
    }
}

impl From<&str> for Message {
    fn from(value: &str) -> Self {
        let bytes = value.as_bytes();
        let chunks = bytes.chunks(8);

        let mut words = chunks.map(|chunk| {
            let mut buf = [0u8; size_of::<usize>()];
            buf[0..chunk.len()].copy_from_slice(chunk);
            usize::from_be_bytes(buf)
        });

        let mut data = [0usize; 4];
        let b_data = &mut data;

        for word in b_data {
            *word = words.next().unwrap_or(0);
        }
        Self { data }
    }
}

impl From<[usize; 4]> for Message {
    fn from(value: [usize; 4]) -> Self {
        Self { data: value }
    }
}

#[allow(clippy::enum_clike_unportable_variant)]
#[derive(Debug)]
#[repr(usize)]
pub enum IpcError {
    Ok = 0,
    WrongRights = 1,
    Full = 2,
    InvalidEndpoint = 3,
    Unknown = usize::MAX,
}

impl From<usize> for IpcError {
    fn from(value: usize) -> Self {
        if (0..=2).contains(&value) {
            unsafe { mem::transmute::<usize, IpcError>(value) }
        } else {
            IpcError::Unknown
        }
    }
}

pub fn derive_cap(cap_id: usize, mask: Rights) -> usize {
    unsafe { syscall2(SYS_DERIVE_CAP, cap_id, mask.0 as usize) }
}

pub fn move_cap(process_id: ProcessId, cap_id: usize) -> usize {
    unsafe { syscall2(SYS_MOVE_CAP, process_id, cap_id) }
}
