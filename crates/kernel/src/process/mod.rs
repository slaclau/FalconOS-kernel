use core::{
    fmt::{Debug, Write},
    sync::atomic::AtomicUsize,
};

use alloc::{collections::btree_map::BTreeMap, vec::Vec};
pub use syscall::process::ProcessId;

use crate::{RING_BUFFER, capability::Capability, log};

pub const KERNEL_TASK_ID: ProcessId = 0;

pub static mut PROCESS_TABLE: Option<BTreeMap<ProcessId, Process>> = Some(BTreeMap::new());
pub static CURRENT_PROCESS_ID: AtomicUsize = AtomicUsize::new(0);

pub static NEXT_ID: AtomicUsize = AtomicUsize::new(0);

pub fn init_multiprocessing() {
    extern "C" fn kernel_task(_arg: usize) -> usize {
        0
    }
    let k = syscall::cap::Cap::<syscall::process::Process>::spawn(kernel_task, 0);
    assert_eq!(k.handle, KERNEL_TASK_ID);
    log!(
        RING_BUFFER,
        "multiprocessing initialized, kernel task running as {k:?}"
    );
}

#[repr(C)]
#[derive(Clone)]
pub struct Process {
    id: ProcessId,
    context: Context,
    stack: Vec<u8>,
    pub exit_code: Option<usize>,
    next_cap: usize,
    caps: BTreeMap<usize, Option<Capability>>,
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

    let ret = func(arg);

    syscall::process::exit(ret);
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
            caps: BTreeMap::new(),
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

    pub fn set_exit_code(&mut self, exit_code: usize) {
        self.exit_code = Some(exit_code)
    }

    pub fn insert_cap(&mut self, cap: Capability) -> Result<usize, &'static str> {
        let key = self.next_cap;
        self.next_cap += 1;
        self.caps.insert(key, Some(cap));
        Ok(key)
    }

    pub fn get_cap(&self, cap_id: usize) -> Result<Capability, &'static str> {
        let cap = self.caps.get(&cap_id).ok_or("error: no cap at this id")?;
        if cap.is_none() {
            Err("error: cap has been moved/revoked")
        } else {
            Ok(cap.clone().unwrap())
        }
    }

    pub fn remove_cap(&mut self, cap_id: usize) -> Result<Capability, &'static str> {
        let cap = self.caps.insert(cap_id, None).unwrap();
        Ok(cap.unwrap())
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
    ) -> Result<usize, &'static str> {
        let cap = self.get_cap(cap_id)?;
        let new_cap = cap.derive(mask)?;
        self.insert_cap(new_cap)
    }

    pub fn move_cap(
        &mut self,
        cap_id: usize,
        target_pid: ProcessId,
    ) -> Result<usize, &'static str> {
        let cap = self.remove_cap(cap_id)?;
        log!(RING_BUFFER, "cap is {cap:?}");
        let proc = Self::get_mut(target_pid);
        proc.insert_cap(cap)
    }

    pub fn get_mut(pid: ProcessId) -> &'static mut Self {
        unsafe { PROCESS_TABLE.as_mut().unwrap().get_mut(&pid).unwrap() }
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

pub unsafe fn switch(old: &mut Process, new: &Process) {
    unsafe { context_switch(&mut old.context, &new.context) };
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

pub fn switch_process(next_id: ProcessId) -> usize {
    unsafe {
        let table = PROCESS_TABLE.as_mut().unwrap();
        let current_id = CURRENT_PROCESS_ID.swap(next_id, core::sync::atomic::Ordering::Relaxed);

        let current: &mut Process = table.get_mut(&current_id).expect("No current process");

        let next: &Process = PROCESS_TABLE
            .as_mut()
            .unwrap()
            .get(&next_id)
            .expect("Invalid next process");
        switch(current, next);
        current_id
    }
}
