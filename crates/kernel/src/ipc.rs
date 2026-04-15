use core::{fmt::Write, sync::atomic::AtomicUsize};

use alloc::collections::btree_map::BTreeMap;

use crate::{RING_BUFFER, log};

pub static mut ENDPOINTS: BTreeMap<EndpointId, Endpoint> = BTreeMap::new();

pub static NEXT_ENDPOINT: AtomicUsize = AtomicUsize::new(0);

pub type EndpointId = usize;

pub struct Endpoint {
    occupied: bool,
    data: Message,
}

impl Endpoint {
    pub fn new() -> Self {
        Self {
            occupied: false,
            data: Message::default(),
        }
    }
    pub fn create() -> EndpointId {
        let ep = Self::new();
        let ep_id = NEXT_ENDPOINT.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        unsafe { ENDPOINTS.insert(ep_id, ep) };
        ep_id
    }
    pub fn write(&mut self, message: Message) -> Result<(), &'static str> {
        if self.occupied {
            Err("endpoint occupied")
        } else {
            self.data = message.clone();
            self.occupied = true;
            Ok(())
        }
    }
    pub fn read(&mut self) -> Result<Message, &'static str> {
        if !self.occupied {
            Err("endpoint empty")
        } else {
            self.occupied = false;
            Ok(self.data.clone())
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Message {
    pub data: [usize; 4],
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
        log!(RING_BUFFER, "got {data:?}");
        Self { data }
    }
}
