#![no_std]

mod arch;

use arch::*;

pub const SYS_SWITCH: usize = 0;
pub const SYS_GET_PID: usize = 1;
pub const SYS_SPAWN: usize = 2;
pub const SYS_EXIT: usize = 3;
pub const SYS_WAIT: usize = 4;
pub const SYS_LOG: usize = 5;

pub const SYS_RECV: usize = 6;
pub const SYS_SEND: usize = 7;

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

pub fn log(message: &str) -> usize {
    unsafe { syscall2(SYS_LOG, message.as_ptr() as usize, message.len()) }
}

pub fn recv(ep_id: usize) -> (usize, Message) {
    let mut msg = Message::default();
    let res = unsafe { syscall5(SYS_RECV, ep_id, &mut msg.data[0] as *mut _ as usize, &mut msg.data[1] as *mut _ as usize, &mut msg.data[2] as *mut _ as usize, &mut msg.data[3] as *mut _ as usize) };
    (res, msg)
}

#[derive(Default)]
pub struct Message {
    pub data: [usize; 4]
}
