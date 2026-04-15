use alloc::vec;
use core::fmt::Write;
use hal::halt;
use syscall::{Message, ProcessId, Rights};

use crate::{
    RING_BUFFER,
    capability::{create_endpoint, derive_cap, move_cap, recv, send},
    ipc::EndpointId,
    log,
    process::{CURRENT_PROCESS_ID, KERNEL_TASK_ID, PROCESS_TABLE, Process, switch_process},
};

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
    switch_process(KERNEL_TASK_ID)
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

pub fn handle_sys_log(start: usize, length: usize) -> usize {
    let bytes = unsafe { core::slice::from_raw_parts(start as *const u8, length) };
    let message = str::from_utf8(bytes).expect("invalid message");
    log!(
        RING_BUFFER,
        "from process {CURRENT_PROCESS_ID:?}: {message} ({start:#x}/{length})"
    );
    0
}

pub fn handle_sys_create_endpoint() -> usize {
    create_endpoint(CURRENT_PROCESS_ID.load(core::sync::atomic::Ordering::Relaxed)).unwrap()
}

pub fn handle_sys_send(ep_id: EndpointId, message: Message) -> usize {
    let res = send(
        CURRENT_PROCESS_ID.load(core::sync::atomic::Ordering::Relaxed),
        ep_id,
        message,
    );
    if res.is_ok() {
        0
    } else {
        res.err().unwrap() as usize
    }
}

pub fn handle_sys_recv(ep_id: EndpointId) -> (EndpointId, Message) {
    let res = recv(
        CURRENT_PROCESS_ID.load(core::sync::atomic::Ordering::Relaxed),
        ep_id,
    );
    if let Ok(msg) = res {
        (0, msg)
    } else {
        (res.err().unwrap() as usize, Message::default())
    }
}

pub fn handle_sys_derive_cap(cap_id: usize, mask: Rights) -> usize {
    derive_cap(
        CURRENT_PROCESS_ID.load(core::sync::atomic::Ordering::Relaxed),
        cap_id,
        mask,
    )
    .unwrap()
}

pub fn handle_sys_move_cap(process_id: ProcessId, cap_id: usize) -> usize {
    move_cap(
        CURRENT_PROCESS_ID.load(core::sync::atomic::Ordering::Relaxed),
        cap_id,
        process_id,
    )
    .unwrap()
}
