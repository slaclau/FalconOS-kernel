use core::{
    fmt::{Debug, Write},
    sync::atomic::AtomicUsize,
};

use alloc::{collections::btree_map::BTreeMap, vec::Vec};
use hal::halt;
pub use syscall::process::ProcessId;
use syscall::{SyscallError, SyscallResult, cap::CapHandle};

use crate::{RING_BUFFER, capability::Capability, log};

pub const KERNEL_TASK_ID: ProcessId = 0;

pub static mut PROCESS_TABLE: Option<BTreeMap<ProcessId, Process>> = Some(BTreeMap::new());
pub static CURRENT_PROCESS_ID: AtomicUsize = AtomicUsize::new(0);

pub static NEXT_ID: AtomicUsize = AtomicUsize::new(0);

pub fn init_multiprocessing() {
    extern "C" fn kernel_task(_arg: usize) -> usize {
        0
    }
    let k = syscall::cap::Cap::<syscall::process::Process>::spawn(kernel_task, 0)
        .expect("Could not spawn kernel task");
    assert_eq!(k.handle, KERNEL_TASK_ID);
    log!(
        RING_BUFFER,
        "multiprocessing initialized, kernel task running as {k:?}"
    );
}

#[repr(C)]
#[derive(Clone)]
pub struct Process {
    pub id: ProcessId,
    context: Context,
    stack: Vec<u8>,
    pub exit_code: Option<CapHandle>,
    next_cap: CapHandle,
    pub blocker: Option<CapHandle>,
    caps: BTreeMap<CapHandle, Option<Capability>>,
    previous: Option<ProcessId>,
}

impl Debug for Process {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Process")
            .field("id", &self.id)
            .field("context", &self.context)
            .finish()
    }
}

#[unsafe(no_mangle)]
extern "C" fn process_entry_trampoline(entry: usize, arg: usize) -> ! {
    let func: fn(usize) -> usize = unsafe { core::mem::transmute(entry) };

    func(arg);

    loop {
        halt();
    }
}

impl Process {
    pub fn new(entry: usize, stack: Vec<u8>, arg: usize) -> Self {
        let id = NEXT_ID.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        let ptr = stack.as_ptr() as usize;
        let stack_top = ptr + stack.len();
        let rsp = stack_top & !0xF;
        Self {
            id,
            stack,
            context: Context {
                rip: process_entry_trampoline as *const () as usize,
                rsp,
                initial_rdi: entry,
                initial_rsi: arg,
                rflags: 0x202,
                ..Default::default()
            },
            exit_code: None,
            next_cap: 0,
            blocker: None,
            caps: BTreeMap::new(),
            previous: None,
        }
    }

    pub fn register(self) -> ProcessId {
        let id = self.id;
        unsafe {
            let table = PROCESS_TABLE.as_mut().unwrap();
            table.insert(id, self);
        }
        id
    }

    #[allow(unused)]
    pub fn exited(self) -> bool {
        self.exit_code.is_some()
    }

    #[allow(unused)]
    pub fn set_exit_code(&mut self, exit_code: usize) {
        self.exit_code = Some(exit_code)
    }

    pub fn r#yield(&self) -> SyscallResult<()> {
        match self.previous {
            Some(pid) => switch_process(pid),
            None => Err(SyscallError::Unknown),
        }
    }

    pub fn insert_cap(&mut self, cap: Capability) -> SyscallResult<CapHandle> {
        let key = self.next_cap;
        self.next_cap += 1;
        self.caps.insert(key, Some(cap));
        Ok(key)
    }

    pub fn get_cap(&self, cap_id: usize) -> SyscallResult<Capability> {
        let cap = self.caps.get(&cap_id);
        match cap {
            Some(Some(cap)) => Ok(cap.clone()),
            _ => Err(SyscallError::NoCap),
        }
    }

    pub fn remove_cap(&mut self, cap_id: usize) -> SyscallResult<Capability> {
        let cap = self.caps.insert(cap_id, None).unwrap();
        match cap {
            Some(cap) => Ok(cap),
            None => Err(SyscallError::NoCap),
        }
    }

    #[allow(unused)]
    pub fn dump_caps(&self) {
        log!(RING_BUFFER, "{self:?}");
        for cap in &self.caps {
            log!(RING_BUFFER, "  {cap:#x?}");
        }
    }

    pub fn derive_cap(
        &mut self,
        cap_id: usize,
        mask: syscall::cap::Rights,
    ) -> Result<CapHandle, SyscallError> {
        let cap = self.get_cap(cap_id)?;
        let new_cap = cap.derive(mask)?;
        self.insert_cap(new_cap)
    }

    pub fn move_cap(
        &mut self,
        cap_id: CapHandle,
        target_process_cap_id: CapHandle,
    ) -> Result<CapHandle, SyscallError> {
        let cap = self.remove_cap(cap_id)?;
        let proc_cap = self.get_cap(target_process_cap_id)?;
        match proc_cap.object {
            crate::capability::KernelObject::Process(pid) => {
                let proc = Self::get_mut(pid);
                proc.insert_cap(cap)
            }
            _ => Err(SyscallError::InvalidObject),
        }
    }

    pub fn get_mut(pid: ProcessId) -> &'static mut Self {
        unsafe { PROCESS_TABLE.as_mut().unwrap().get_mut(&pid).unwrap() }
    }

    pub fn get_current_mut() -> &'static mut Self {
        let current_pid = CURRENT_PROCESS_ID.load(core::sync::atomic::Ordering::Relaxed);
        Self::get_mut(current_pid)
    }
}
#[derive(Debug, Default, Clone, Copy)]
#[repr(C)]
pub struct Context {
    pub r15: usize,
    pub r14: usize,
    pub r13: usize,
    pub r12: usize,
    pub rbx: usize,
    pub rbp: usize,

    pub rip: usize,
    pub rsp: usize,

    pub rflags: usize,

    pub initial_rdi: usize,
    pub initial_rsi: usize,
}

pub unsafe fn switch(old: &mut Process, new: &mut Process) -> SyscallResult<()> {
    new.previous = Some(old.id);
    unsafe { context_switch(&mut old.context, &new.context) };
    Ok(())
}

#[unsafe(naked)]
pub unsafe extern "C" fn context_switch(_old: *mut Context, _new: *const Context) {
    core::arch::naked_asm!(
        "
        // rdi = old
        // rsi = new
        
        mov [rdi + {offset_r15}], r15
        mov [rdi + {offset_r14}], r14
        mov [rdi + {offset_r13}], r13
        mov [rdi + {offset_r12}], r12
        mov [rdi + {offset_rbx}], rbx
        mov [rdi + {offset_rbp}], rbp

        mov [rdi + {offset_rsp}], rsp

        pushfq
        pop rax
        mov [rdi + {offset_rflags}], rax

        lea rax, [rip + .resume]
        mov [rdi + {offset_rip}], rax

        mov r15, [rsi + {offset_r15}]
        mov r14, [rsi + {offset_r14}]
        mov r13, [rsi + {offset_r13}]
        mov r12, [rsi + {offset_r12}]
        mov rbx, [rsi + {offset_rbx}]
        mov rbp, [rsi + {offset_rbp}]

        mov rsp, [rsi + {offset_rsp}]

        mov rax, [rsi + {offset_rflags}]
        push rax
        popfq

        mov rax, [rsi + {offset_rip}]
        mov rdi, [rsi + {offset_initial_rdi}]
        mov rsi, [rsi + {offset_initial_rsi}]
        jmp rax

        .resume:
        ret
        ",
        offset_r15 = const core::mem::offset_of!(Context, r15),
        offset_r14 = const core::mem::offset_of!(Context, r14),
        offset_r13 = const core::mem::offset_of!(Context, r13),
        offset_r12 = const core::mem::offset_of!(Context, r12),
        offset_rbx = const core::mem::offset_of!(Context, rbx),
        offset_rbp = const core::mem::offset_of!(Context, rbp),
        offset_rip = const core::mem::offset_of!(Context, rip),
        offset_rsp = const core::mem::offset_of!(Context, rsp),
        offset_rflags = const core::mem::offset_of!(Context, rflags),
        offset_initial_rsi = const core::mem::offset_of!(Context, initial_rsi),
        offset_initial_rdi = const core::mem::offset_of!(Context, initial_rdi),
    )
}

pub fn switch_process(next_id: ProcessId) -> SyscallResult<()> {
    unsafe {
        let current: &mut Process = Process::get_current_mut();
        CURRENT_PROCESS_ID.swap(next_id, core::sync::atomic::Ordering::Relaxed);
        let next: &mut Process = Process::get_current_mut();
        switch(current, next)
    }
}
