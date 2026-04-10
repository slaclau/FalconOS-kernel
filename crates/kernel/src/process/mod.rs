use core::{fmt::Debug, sync::atomic::AtomicUsize};

use alloc::{collections::btree_map::BTreeMap, vec::Vec};

pub static mut PROCESS_TABLE: Option<BTreeMap<ProcessId, Process>> = Some(BTreeMap::new());
pub static CURRENT_PROCESS_ID: AtomicUsize = AtomicUsize::new(0);

pub static NEXT_ID: AtomicUsize = AtomicUsize::new(0);

#[repr(C)]
pub struct Process {
    id: ProcessId,
    context: Context,
    stack: Vec<u8>,
}

impl Debug for Process {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Process")
            .field("id", &self.id)
            .field("context", &self.context)
            .finish()
    }
}

impl Process {
    pub fn new(entry: extern "C" fn(), stack: Vec<u8>) -> Self {
        let id = NEXT_ID.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        let ptr = stack.as_ptr() as usize;
        let stack_top = ptr + stack.len();
        let rsp = stack_top & !0xF;
        Self {
            id,
            stack,
            context: Context {
                rip: entry as usize,
                rsp,
                rflags: 0x202,
                ..Default::default()
            },
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
}

pub type ProcessId = usize;

#[derive(Debug, Default)]
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
    )
}

pub fn switch_process(next_id: ProcessId) {
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
    }
}
