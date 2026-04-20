use core::{
    cmp::{max, min},
    fmt::Debug,
};

use syscall::{SyscallError, cap::Rights};

use crate::{
    PhysicalAddress,
    ipc::EndpointId,
    process::{KERNEL_TASK_ID, PROCESS_TABLE, ProcessId},
};

#[allow(unused)]
#[derive(Clone, Debug)]
pub enum KernelObject {
    Untyped { addr: PhysicalAddress, size: usize },
    Process(ProcessId),
    Endpoint(EndpointId),
    ReplyEndpoint(EndpointId),
    Frame { addr: PhysicalAddress, size: usize },
}

#[derive(Clone, Debug)]
pub struct Capability {
    pub object: KernelObject,
    pub rights: Rights,
}

impl Capability {
    pub fn derive(self, mask: Rights) -> Result<Self, SyscallError> {
        if !self.rights.grant() {
            return Err(SyscallError::RightsError(syscall::RightsError::NoGrant));
        }
        Ok(Self {
            object: self.object,
            rights: self.rights & mask,
        })
    }
    pub fn has_rights(self, rights: Rights) -> Result<(), SyscallError> {
        if self.rights.matches(rights) {
            Ok(())
        } else {
            Err(SyscallError::RightsError(syscall::RightsError::Unknown))
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
                kernel_task
                    .insert_cap(cap)
                    .expect("Could not insert initial cap");
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
                kernel_task
                    .insert_cap(cap)
                    .expect("Could not insert initial cap");
            }
        }
    }
}
