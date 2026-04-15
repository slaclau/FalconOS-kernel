use core::sync::atomic::AtomicUsize;

use alloc::collections::btree_map::BTreeMap;
use syscall::{IpcError, Message};

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
    pub fn write(&mut self, message: Message) -> Result<(), IpcError> {
        if self.occupied {
            Err(IpcError::Full)
        } else {
            self.data = message;
            self.occupied = true;
            Ok(())
        }
    }
    pub fn read(&mut self) -> Result<Message, IpcError> {
        if !self.occupied {
            Err(IpcError::Full)
        } else {
            self.occupied = false;
            Ok(self.data)
        }
    }
}
