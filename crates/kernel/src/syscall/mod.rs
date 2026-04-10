use alloc::vec;

use crate::process::{CURRENT_PROCESS_ID, Process, switch_process};

pub fn handle_sys_switch(next_id: usize) -> usize {
    switch_process(next_id)
}

pub fn handle_sys_get_pid() -> usize {
    CURRENT_PROCESS_ID.load(core::sync::atomic::Ordering::Relaxed)
}

pub const STACK_SIZE: usize = 4096 * 2;
pub fn handle_sys_spawn(entry: usize, arg: usize) -> usize {
    let stack = vec![0; STACK_SIZE];
    let process = Process::new(entry, stack, arg);
    process.register()
}
