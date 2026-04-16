use core::mem;

use crate::{
    cap::{Cap, CapHandle, CapType},
    *,
};

#[derive(Clone, Copy)]
pub struct Endpoint;
pub struct ReplyEndpoint;
impl CapType for Endpoint {}
impl CapType for ReplyEndpoint {}

impl Cap<Endpoint> {
    pub fn create() -> Result<Cap<Endpoint>, &'static str> {
        let res = unsafe { syscall0(SYS_CREATE_ENDPOINT) };
        Ok(Cap::<Endpoint>::new(res))
    }

    pub fn send(self, message: Message) -> Result<Message, IpcError> {
        send(self.handle, message)
    }

    pub fn recv(self) -> Result<(Cap<ReplyEndpoint>, Message), IpcError> {
        let (handle, msg) = recv(self.handle)?;
        Ok((Cap::new(handle), msg))
    }
}

impl Cap<ReplyEndpoint> {
    pub fn reply(self, message: Message) -> Result<(), IpcError> {
        reply(self.handle, message)
    }
}

fn send(ep_id: CapHandle, mut message: Message) -> Result<Message, IpcError> {
    let (code, words) = unsafe {
        out_syscall5(
            SYS_SEND,
            ep_id,
            message.data[0],
            message.data[1],
            message.data[2],
            message.data[3],
        )
    };
    message.data = *words[1..5].as_array().expect("Not able to unwrap");
    if code == 0 {
        Ok(message)
    } else {
        Err(code.into())
    }
}

fn recv(ep_id: CapHandle) -> Result<(CapHandle, Message), IpcError> {
    let (code, words) = unsafe { out_syscall5(SYS_RECV, ep_id, 0, 0, 0, 0) };

    if code == 0 {
        Ok((
            code,
            Message {
                data: *words[1..5].as_array().unwrap(),
            },
        ))
    } else {
        Err(code.into())
    }
}

fn reply(ep_id: CapHandle, message: Message) -> Result<(), IpcError> {
    let code = unsafe {
        syscall5(
            SYS_RECV,
            ep_id,
            message.data[0],
            message.data[1],
            message.data[2],
            message.data[3],
        )
    };

    if code == 0 { Ok(()) } else { Err(code.into()) }
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
    Empty = 2,
    Full = 3,
    InvalidEndpoint = 4,
    Unknown = usize::MAX,
}

impl From<usize> for IpcError {
    fn from(value: usize) -> Self {
        if (0..=4).contains(&value) {
            unsafe { mem::transmute::<usize, IpcError>(value) }
        } else {
            IpcError::Unknown
        }
    }
}
