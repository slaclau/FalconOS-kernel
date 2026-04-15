use core::sync::atomic::AtomicUsize;

use alloc::collections::btree_map::BTreeMap;
use syscall::Message;

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
