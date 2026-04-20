use alloc::vec;
use core::fmt::Write;
use syscall::{
    SyscallError,
    cap::{CapHandle, Rights},
    ipc::{IpcStatus, Message},
};

use crate::{
    RING_BUFFER,
    capability::{Capability, KernelObject},
    ipc::{ENDPOINTS, Endpoint},
    log,
    process::{CURRENT_PROCESS_ID, Process, switch_process},
};

pub type SyscallReturn = Result<[usize; 6], SyscallError>;

pub fn handle_sys_switch(cap_id: usize) -> SyscallReturn {
    let proc = Process::get_mut(CURRENT_PROCESS_ID.load(core::sync::atomic::Ordering::Relaxed));
    let cap = proc.get_cap(cap_id)?;

    match cap.object {
        KernelObject::Process(next_id) => {
            if cap.has_rights(Rights::EXEC).is_ok() {
                switch_process(next_id).map(|_| [0; 6])
            } else {
                Err(SyscallError::RightsError(syscall::RightsError::NoExec))
            }
        }
        _ => Err(SyscallError::InvalidObject),
    }
}

pub fn handle_sys_get_pid() -> SyscallReturn {
    Ok([
        CURRENT_PROCESS_ID.load(core::sync::atomic::Ordering::Relaxed),
        0,
        0,
        0,
        0,
        0,
    ])
}

pub const STACK_SIZE: usize = 4096 * 2;
pub fn handle_sys_spawn(entry: usize, arg: usize) -> SyscallReturn {
    let stack = vec![0; STACK_SIZE];
    let process = Process::new(entry, stack, arg);
    let pid = process.register();
    let cap = Capability {
        object: KernelObject::Process(pid),
        rights: Rights::ALL,
    };
    Process::get_mut(CURRENT_PROCESS_ID.load(core::sync::atomic::Ordering::Relaxed))
        .insert_cap(cap)
        .map(|handle| [handle, 0, 0, 0, 0, 0])
}

pub fn handle_sys_yield() -> SyscallReturn {
    let proc = Process::get_current_mut();
    proc.r#yield().map(|_| [0; 6])
}

pub fn handle_sys_log(start: usize, length: usize) -> SyscallReturn {
    let bytes = unsafe { core::slice::from_raw_parts(start as *const u8, length) };
    let message = str::from_utf8(bytes).expect("invalid message");
    log!(
        RING_BUFFER,
        "from process {CURRENT_PROCESS_ID:?}: {message} ({start:#x}/{length})"
    );
    Ok([0; 6])
}

pub fn handle_sys_derive_cap(cap_id: CapHandle, mask: Rights) -> SyscallReturn {
    Process::get_current_mut()
        .derive_cap(cap_id, mask)
        .map(|handle| [handle, 0, 0, 0, 0, 0])
}

pub fn handle_sys_move_cap(process_cap_id: CapHandle, cap_id: CapHandle) -> SyscallReturn {
    Process::get_current_mut()
        .move_cap(cap_id, process_cap_id)
        .map(|handle| [handle, 0, 0, 0, 0, 0])
}

pub fn handle_sys_create_endpoint() -> SyscallReturn {
    let ep_id = Endpoint::create();
    let cap = Capability {
        object: KernelObject::Endpoint(ep_id),
        rights: Rights::ALL,
    };
    Process::get_current_mut()
        .insert_cap(cap)
        .map(|handle| [handle, 0, 0, 0, 0, 0])
}

pub fn handle_sys_send(cap_id: CapHandle, message: Message) -> SyscallReturn {
    let proc = Process::get_current_mut();
    proc.blocker = Some(cap_id);
    let cap = proc.get_cap(cap_id)?;

    let endpoint: &mut Endpoint = match cap.object {
        KernelObject::Endpoint(endpoint_id) => {
            if cap.has_rights(Rights::WRITE).is_ok() {
                unsafe {
                    ENDPOINTS
                        .get_mut(&endpoint_id)
                        .ok_or(SyscallError::IpcError(
                            syscall::ipc::IpcError::InvalidEndpoint,
                        ))
                }
            } else {
                Err(SyscallError::RightsError(syscall::RightsError::NoWrite))
            }
        }
        _ => Err(SyscallError::InvalidObject),
    }?;

    let res = endpoint.write(message, proc).map(|status| {
        [
            unsafe { core::mem::transmute::<IpcStatus, usize>(status) },
            0,
            0,
            0,
            0,
            0,
        ]
    });
    if let Some(receiver) = endpoint.waiting_receiver.take() {
        log!(
            RING_BUFFER,
            "receiver waiting, switch immediately to {receiver}"
        );
        switch_process(receiver).expect("failed to switch to receiver");
    }
    res
}

pub fn handle_sys_recv(cap_id: CapHandle) -> SyscallReturn {
    let proc = Process::get_current_mut();
    let cap = proc.get_cap(cap_id)?;
    let (handle, endpoint) = match cap.object {
        KernelObject::Endpoint(endpoint_id) => {
            if cap.has_rights(Rights::READ).is_ok() {
                let reply_cap = Capability {
                    object: KernelObject::ReplyEndpoint(endpoint_id),
                    rights: Rights::WRITE,
                };
                let handle = proc.insert_cap(reply_cap)?;
                let endpoint = unsafe {
                    ENDPOINTS
                        .get_mut(&endpoint_id)
                        .ok_or(SyscallError::IpcError(
                            syscall::ipc::IpcError::InvalidEndpoint,
                        ))
                }?;
                Ok((handle, endpoint))
            } else {
                Err(SyscallError::RightsError(syscall::RightsError::NoWrite))
            }
        }
        _ => Err(SyscallError::InvalidObject),
    }?;

    let words: [usize; 4] = endpoint.read(proc).map(|msg| msg.into())?;
    Ok([handle, words[0], words[1], words[2], words[3], 0])
}

pub fn handle_sys_reply(cap_id: CapHandle, message: Message) -> SyscallReturn {
    let proc = Process::get_current_mut();
    proc.blocker = Some(cap_id);
    let cap = proc.get_cap(cap_id)?;

    let endpoint: &mut Endpoint = match cap.object {
        KernelObject::ReplyEndpoint(endpoint_id) => {
            if cap.has_rights(Rights::WRITE).is_ok() {
                proc.remove_cap(cap_id).expect("Could not remove reply cap");
                unsafe {
                    ENDPOINTS
                        .get_mut(&endpoint_id)
                        .ok_or(SyscallError::IpcError(
                            syscall::ipc::IpcError::InvalidEndpoint,
                        ))
                }
            } else {
                Err(SyscallError::RightsError(syscall::RightsError::NoWrite))
            }
        }
        _ => Err(SyscallError::InvalidObject),
    }?;

    let res = endpoint.reply(message).map(|_| [0; 6]);

    if let Some(sender) = endpoint.waiting_sender.take() {
        log!(
            RING_BUFFER,
            "sender waiting, switch immediately to {sender}"
        );
        switch_process(sender).expect("failed to switch to sender");
    }
    res
}
