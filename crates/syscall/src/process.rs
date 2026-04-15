use crate::*;

pub type ProcessId = usize;

pub fn switch(process_id: ProcessId) -> ProcessId {
    unsafe { syscall1(SYS_SWITCH, process_id) }
}

pub fn get_pid() -> ProcessId {
    unsafe { syscall0(SYS_GET_PID) }
}

pub fn spawn(entry: extern "C" fn(arg: usize) -> usize, arg: usize) -> ProcessId {
    unsafe { syscall2(SYS_SPAWN, entry as usize, arg) }
}

pub fn exit(exit_code: usize) -> ! {
    unsafe { syscall1(SYS_EXIT, exit_code) };
    unreachable!()
}

pub fn wait(pid: ProcessId) -> usize {
    unsafe { syscall1(SYS_WAIT, pid) }
}
