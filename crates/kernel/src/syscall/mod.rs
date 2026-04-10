use alloc::vec;
use hal::halt;

use crate::process::{CURRENT_PROCESS_ID, PROCESS_TABLE, Process, switch_process};

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

pub fn handle_sys_exit(exit_code: usize) -> usize {
    let id = CURRENT_PROCESS_ID.load(core::sync::atomic::Ordering::Relaxed);
    unsafe {
        let table = PROCESS_TABLE.as_mut().unwrap();
        let proc = table.get_mut(&id).expect("Invalid process");
        proc.set_exit_code(exit_code);
    }
    exit_code
}

pub fn handle_sys_wait(pid: usize) -> usize {
    unsafe {
        let table = PROCESS_TABLE.as_mut().unwrap();

        loop {
            let proc = table.get_mut(&pid).expect("Invalid process");
            if let Some(code) = proc.exit_code {
                return code;
            }
            halt();
        }
    }
}