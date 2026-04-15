use crate::{
    cap::{Cap, CapHandle, CapType},
    *,
};

pub type ProcessId = usize;

#[derive(Clone, Copy, Debug)]
pub struct Process {}
impl CapType for Process {}

impl Cap<Process> {
    pub fn spawn(entry: extern "C" fn(arg: usize) -> usize, arg: usize) -> Self {
        let handle = spawn(entry, arg);
        Self::new(handle)
    }
    pub fn switch(self) -> Self {
        let handle = switch(self.handle);
        Self::new(handle)
    }
}

fn switch(process_cap: CapHandle) -> CapHandle {
    unsafe { syscall1(SYS_SWITCH, process_cap) }
}

pub fn get_pid() -> ProcessId {
    unsafe { syscall0(SYS_GET_PID) }
}

fn spawn(entry: extern "C" fn(arg: usize) -> usize, arg: usize) -> CapHandle {
    unsafe { syscall2(SYS_SPAWN, entry as usize, arg) }
}

pub fn exit(exit_code: usize) -> ! {
    unsafe { syscall1(SYS_EXIT, exit_code) };
    unreachable!()
}

pub fn wait(pid: ProcessId) -> usize {
    unsafe { syscall1(SYS_WAIT, pid) }
}
