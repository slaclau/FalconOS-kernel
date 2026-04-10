#![no_std]

mod arch;

use arch::*;

pub const SYS_SWITCH: usize = 0;
pub const SYS_GET_PID: usize = 1;
pub const SYS_SPAWN: usize = 2;
pub const SYS_EXIT: usize = 3;
pub const SYS_WAIT: usize = 4;

pub fn switch(process_id: usize) -> usize {
    unsafe { syscall1(SYS_SWITCH, process_id) }
}

pub fn get_pid() -> usize {
    unsafe { syscall0(SYS_GET_PID) }
}

pub fn spawn(entry: extern "C" fn(usize) -> usize, arg: usize) -> usize {
    unsafe { syscall2(SYS_SPAWN, entry as usize, arg) }
}

pub fn exit(exit_code: usize) -> ! {
    unsafe { syscall1(SYS_EXIT, exit_code) };
    unreachable!()
}

pub fn wait(pid: usize) -> usize {
    unsafe { syscall1(SYS_WAIT, pid) }
}
