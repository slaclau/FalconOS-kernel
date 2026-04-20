use core::sync::atomic::Atomic;

use alloc::collections::btree_map::BTreeMap;
use syscall::{
    SyscallError,
    ipc::{IpcStatus, Message},
    process::ProcessId,
};

use crate::process::Process;

pub type EndpointId = usize;
pub static mut ENDPOINTS: BTreeMap<EndpointId, Endpoint> = BTreeMap::new();
pub static NEXT_ENDPOINT_ID: Atomic<EndpointId> = Atomic::<EndpointId>::new(0);

pub struct Endpoint {
    message: Option<Message>,
    pub waiting_sender: Option<ProcessId>,
    pub waiting_receiver: Option<ProcessId>,
}

impl Endpoint {
    fn new() -> Self {
        Self {
            message: None,
            waiting_sender: None,
            waiting_receiver: None,
        }
    }

    pub fn create() -> EndpointId {
        let ep = Self::new();
        let ep_id = NEXT_ENDPOINT_ID.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        unsafe { ENDPOINTS.insert(ep_id, ep) };
        ep_id
    }

    pub fn write(
        &mut self,
        message: Message,
        process: &'static Process,
    ) -> Result<IpcStatus, SyscallError> {
        if let Some(pid) = self.waiting_receiver {
            let proc = Process::get_mut(pid);
            proc.blocker = None
        }
        match self.message {
            Some(_) => Err(SyscallError::IpcError(syscall::ipc::IpcError::Full)),
            None => {
                self.message = Some(message);
                self.waiting_sender = Some(process.id);
                if self.waiting_receiver.is_some() {
                    Ok(IpcStatus::Ready)
                } else {
                    Ok(IpcStatus::WouldBlock)
                }
            }
        }
    }

    pub fn read(&mut self, process: &'static Process) -> Result<Message, SyscallError> {
        if let Some(pid) = self.waiting_sender {
            let proc = Process::get_mut(pid);
            proc.blocker = None
        }
        match self.message {
            Some(_) => {
                self.waiting_receiver = Some(process.id);
                Ok(self.message.take().unwrap())
            }
            None => Err(SyscallError::IpcError(syscall::ipc::IpcError::Empty)),
        }
    }

    pub fn reply(&mut self, message: Message) -> Result<(), SyscallError> {
        match self.message {
            Some(_) => Err(SyscallError::IpcError(syscall::ipc::IpcError::Full)),
            None => {
                self.message = Some(message);
                Ok(())
            }
        }
    }
}
