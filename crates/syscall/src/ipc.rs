use core::mem;

use crate::{
    cap::{Cap, CapHandle, CapType},
    *,
};

#[derive(Clone, Copy, Debug)]
pub struct Endpoint;

#[derive(Clone, Copy, Debug)]
pub struct ReplyEndpoint;
impl CapType for Endpoint {}
impl CapType for ReplyEndpoint {}

impl Cap<Endpoint> {
    pub fn create() -> SyscallResult<Cap<Endpoint>> {
        let res = unsafe { syscall0(SYS_CREATE_ENDPOINT) }.map(|words| words[0])?;
        Ok(Cap::<Endpoint>::new(res))
    }

    pub fn send(self, message: Message) -> SyscallResult<IpcStatus> {
        send(self.handle, message)
    }

    pub fn recv(self) -> SyscallResult<(Cap<ReplyEndpoint>, Message)> {
        let (handle, msg) = recv(self.handle)?;
        Ok((Cap::new(handle), msg))
    }
}

impl Cap<ReplyEndpoint> {
    pub fn reply(self, message: Message) -> SyscallResult<()> {
        reply(self.handle, message)
    }
}

fn send(ep_id: CapHandle, message: Message) -> SyscallResult<IpcStatus> {
    unsafe {
        syscall5(
            SYS_SEND,
            ep_id,
            message.data[0],
            message.data[1],
            message.data[2],
            message.data[3],
        )
    }
    .map(|words| unsafe { mem::transmute::<usize, IpcStatus>(words[0]) })
}

fn recv(ep_id: CapHandle) -> SyscallResult<(CapHandle, Message)> {
    unsafe { syscall1(SYS_RECV, ep_id) }
        .map(|words| (words[0], (*words[1..5].as_array::<4>().unwrap()).into()))
}

fn reply(ep_id: CapHandle, message: Message) -> SyscallResult<()> {
    unsafe {
        syscall5(
            SYS_REPLY,
            ep_id,
            message.data[0],
            message.data[1],
            message.data[2],
            message.data[3],
        )
    }
    .map(|_| ())
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

impl From<[u8; 32]> for Message {
    fn from(value: [u8; 32]) -> Self {
        let mut buf = [0usize; 4];
        let words = value
            .chunks(8)
            .map(|chunk| usize::from_be_bytes(*chunk.as_array().unwrap()));
        for (i, word) in words.enumerate() {
            buf[i] = word;
        }

        buf.into()
    }
}

impl From<[usize; 4]> for Message {
    fn from(value: [usize; 4]) -> Self {
        Self { data: value }
    }
}

impl From<Message> for [usize; 4] {
    fn from(value: Message) -> Self {
        value.data
    }
}

#[derive(Debug)]
#[repr(C)]
pub enum IpcError {
    Empty,
    Full,
    InvalidEndpoint,
}

#[derive(Debug)]
#[repr(usize)]
pub enum IpcStatus {
    WouldBlock,
    Ready,
}
