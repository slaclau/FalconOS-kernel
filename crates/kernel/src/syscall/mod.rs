use crate::process::{CURRENT_PROCESS_ID, switch_process};

pub fn handle_sys_switch(next_id: usize) -> usize {
    switch_process(next_id);
    next_id
}

pub fn handle_sys_get_pid() -> usize {
    CURRENT_PROCESS_ID.load(core::sync::atomic::Ordering::Relaxed)
}