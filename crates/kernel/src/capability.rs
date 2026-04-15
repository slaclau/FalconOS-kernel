use core::{
    cmp::{max, min},
    fmt::Debug,
};

use syscall::{
    cap::Rights,
    ipc::{IpcError, Message},
};

use crate::{
    PhysicalAddress,
    ipc::{ENDPOINTS, Endpoint, EndpointId},
    process::{KERNEL_TASK_ID, PROCESS_TABLE, Process, ProcessId},
};

#[allow(unused)]
#[derive(Clone, Debug)]
pub enum KernelObject {
    Untyped { addr: PhysicalAddress, size: usize },
    Process(ProcessId),
    Endpoint(EndpointId),
    Frame { addr: PhysicalAddress, size: usize },
}

#[derive(Clone, Debug)]
pub struct Capability {
    pub object: KernelObject,
    pub rights: Rights,
}

impl Capability {
    pub fn derive(self, mask: Rights) -> Result<Self, &'static str> {
        if !self.rights.grant() {
            return Err("does not have grant rights");
        }
        Ok(Self {
            object: self.object,
            rights: self.rights & mask,
        })
    }
    pub fn has_rights(self, rights: Rights) -> Result<(), &'static str> {
        if self.rights.matches(rights) {
            Ok(())
        } else {
            Err("invalid rights")
        }
    }
}

pub fn init(
    kernel_memory_region: Option<[PhysicalAddress; 2]>,
    available_memory_regions: [Option<[PhysicalAddress; 2]>; 16],
) {
    let kernel_task = unsafe {
        let table = PROCESS_TABLE.as_mut().unwrap();
        table.get_mut(&KERNEL_TASK_ID).unwrap()
    };
    let reserved = kernel_memory_region.unwrap();
    for region in available_memory_regions.into_iter().flatten() {
        if region[0] < reserved[0] {
            let upper = min(region[1], reserved[0]);
            let size = upper - region[0];
            if size > 0 {
                let cap = Capability {
                    object: KernelObject::Untyped {
                        addr: region[0],
                        size,
                    },
                    rights: Rights::RWE,
                };
                kernel_task.insert_cap(cap).unwrap();
            }
        }
        if region[1] > reserved[1] {
            let lower = max(region[0], reserved[1]);
            let size = region[1] - lower;
            if size > 0 {
                let cap = Capability {
                    object: KernelObject::Untyped { addr: lower, size },
                    rights: Rights::RWE,
                };
                kernel_task.insert_cap(cap).unwrap();
            }
        }
    }
}

pub fn create_endpoint(pid: ProcessId) -> Result<usize, &'static str> {
    let ep_id = Endpoint::create();
    let proc = Process::get_mut(pid);
    let cap = Capability {
        object: KernelObject::Endpoint(ep_id),
        rights: Rights::ALL,
    };
    proc.insert_cap(cap)
}

pub fn derive_cap(pid: ProcessId, cap_id: usize, mask: Rights) -> Result<usize, &'static str> {
    let proc = Process::get_mut(pid);
    proc.derive_cap(cap_id, mask)
}

pub fn move_cap(
    source_pid: ProcessId,
    source_cap_id: usize,
    target_pid: ProcessId,
) -> Result<usize, &'static str> {
    let proc = Process::get_mut(source_pid);
    proc.move_cap(source_cap_id, target_pid)
}

fn get_endpoint(endpoint_id: EndpointId) -> Result<&'static mut Endpoint, &'static str> {
    unsafe {
        ENDPOINTS
            .get_mut(&endpoint_id)
            .ok_or("no endpoint with this id")
    }
}

pub fn send(pid: ProcessId, cap_id: usize, message: Message) -> Result<(), IpcError> {
    let proc = Process::get_mut(pid);
    let cap = proc.get_cap(cap_id).or(Err(IpcError::InvalidEndpoint))?;

    match cap.object {
        KernelObject::Endpoint(endpoint_id) => {
            cap.has_rights(Rights::WRITE)
                .or(Err(IpcError::WrongRights))?;
            let ep = get_endpoint(endpoint_id).unwrap();
            ep.write(message)
        }
        _ => Err(IpcError::InvalidEndpoint),
    }
}

pub fn recv(pid: ProcessId, cap_id: usize) -> Result<Message, IpcError> {
    let proc = Process::get_mut(pid);
    let cap = proc.get_cap(cap_id).or(Err(IpcError::InvalidEndpoint))?;

    match cap.object {
        KernelObject::Endpoint(endpoint_id) => {
            cap.has_rights(Rights::READ)
                .or(Err(IpcError::WrongRights))?;
            get_endpoint(endpoint_id).unwrap().read()
        }
        _ => Err(IpcError::InvalidEndpoint),
    }
}
