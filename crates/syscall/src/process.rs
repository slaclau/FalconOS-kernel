use crate::{
    cap::{Cap, CapHandle, CapType},
    *,
};

pub type ProcessId = usize;

#[derive(Clone, Copy, Debug)]
pub struct Process {}
impl CapType for Process {}

impl Cap<Process> {
    pub fn spawn(entry: extern "C" fn(arg: usize) -> usize, arg: usize) -> SyscallResult<Self> {
        let handle = spawn(entry, arg)?;
        Ok(Self::new(handle))
    }
    pub fn switch(self) -> SyscallResult<()> {
        switch(self.handle)
    }
}

fn switch(process_cap: CapHandle) -> SyscallResult<()> {
    unsafe { syscall1(SYS_SWITCH, process_cap) }.map(|_| ())
}

fn spawn(entry: extern "C" fn(arg: usize) -> usize, arg: usize) -> SyscallResult<CapHandle> {
    unsafe { syscall2(SYS_SPAWN, entry as usize, arg) }.map(|words| words[0])
}

pub fn get_pid() -> SyscallResult<ProcessId> {
    unsafe { syscall0(SYS_GET_PID) }.map(|words| words[0])
}

pub fn r#yield() -> SyscallResult<()> {
    unsafe { syscall0(SYS_YIELD) }.map(|_| ())
}

#[derive(Debug)]
#[repr(C)]
pub enum ProcessError {
    Unknown,
}
