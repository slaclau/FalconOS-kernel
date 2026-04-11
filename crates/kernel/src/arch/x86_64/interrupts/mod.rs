use core::{arch::asm, fmt::Write, sync::atomic::AtomicUsize};

use hal::{self};
use macros::{assign_handlers, make_handlers};
use spin::{Mutex, Once};
use syscall::{SYS_EXIT, SYS_GET_PID, SYS_LOG, SYS_SPAWN, SYS_SWITCH, SYS_WAIT};

use crate::{
    DEBUG_WRITER, RING_BUFFER,
    arch::x86_64::{
        pic::ChainedPics,
        segmentation::tss,
        tables::{
            gdt::{self, load_tss},
            idt::{self, PageFaultErrorCode},
        },
    },
    log,
    syscall::{
        handle_sys_exit, handle_sys_get_pid, handle_sys_log, handle_sys_spawn, handle_sys_switch,
        handle_sys_wait,
    },
};

static IDT: Once<idt::Table> = Once::new();
static GDT: Once<(
    gdt::Table<16>,
    gdt::SegmentSelector,
    gdt::SegmentSelector,
    gdt::SegmentSelector,
    gdt::SegmentSelector,
)> = Once::new();
static TSS: Once<tss::TaskStateSegment> = Once::new();

static PICS: Once<Mutex<ChainedPics>> = Once::new();
const DOUBLE_FAULT_STACK_INDEX: u16 = 0;

const TIMER_IRQ: u64 = 0x20;
const SYSCALL_IRQ: u64 = 0x80;

static COUNTER: AtomicUsize = AtomicUsize::new(0);

make_handlers!();

pub fn configure() {
    log!(RING_BUFFER, "interrupts::configure called");

    create_and_load_gdt();
    create_and_load_idt();

    enable_timer();
}

fn create_and_load_gdt() {
    TSS.call_once(|| {
        let mut tss = tss::TaskStateSegment::new();
        tss.ist[DOUBLE_FAULT_STACK_INDEX as usize] = {
            const STACK_SIZE: usize = 4096 * 4;
            static mut STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];
            let stack_start = &raw const STACK as usize;
            let stack_end = stack_start + STACK_SIZE;

            stack_end as u64
        };

        tss
    });
    GDT.call_once(|| {
        let mut gdt = gdt::Table::empty();
        let kcode_selector = gdt.append(gdt::Descriptor::kernel_code_segment());
        let kdata_selector = gdt.append(gdt::Descriptor::kernel_data_segment());
        let udata_selector = gdt.append(gdt::Descriptor::user_data_segment());
        let tss_selector = gdt.append(gdt::Descriptor::tss_segment(TSS.get().unwrap()));

        (
            gdt,
            kcode_selector,
            kdata_selector,
            udata_selector,
            tss_selector,
        )
    });

    unsafe { GDT.get().unwrap().0.load() };

    log!(RING_BUFFER, "loaded GDT");

    unsafe {
        asm!(
            "push {sel}",
            "lea {tmp}, [55f + rip]",
            "push {tmp}",
            "retfq",
            "55:",
            sel = in(reg) GDT.get().unwrap().1.as_u64(),
            tmp = lateout(reg) _,
            options(preserves_flags),
        );

        load_tss(GDT.get().unwrap().4);
    }
    log!(RING_BUFFER, "loaded TSS");
}

fn create_and_load_idt() {
    IDT.call_once(|| {
        let mut idt = idt::Table::new();

        unsafe {
            idt.page_pault()
                .set_handler_addr(page_fault_handler as *const () as u64)
                .options
                .set_present(true);
            idt.gp_fault()
                .set_handler_addr(gp_fault_handler as *const () as u64)
                .options
                .set_present(true);
            idt.double_fault()
                .set_handler_addr(double_fault_handler as *const () as u64)
                .options
                .set_present(true)
                .set_stack_index(DOUBLE_FAULT_STACK_INDEX as u8);

            assign_handlers!();

            idt.interrupts()[SYSCALL_IRQ as usize - 32]
                .set_handler_addr(syscall_entry as *const () as u64)
        };

        idt
    });

    unsafe { IDT.get().unwrap().load() };

    log!(RING_BUFFER, "loaded IDT");
}

fn enable_timer() {
    PICS.call_once(|| {
        let mut pics = unsafe { ChainedPics::new(32, 40) };
        unsafe { pics.initialize() };
        Mutex::new(pics)
    });
    unsafe { PICS.get().unwrap().lock().write_masks(0xFE, 0xFF) };
    log!(RING_BUFFER, "enabled timer (PIT) and masked other IRQs");

    hal::interrupts::enable(true);
    let enabled = hal::interrupts::are_enabled();
    log!(RING_BUFFER, "enabled interrupts {enabled}");
}

fn _test_page_fault() {
    unsafe {
        *(0xdeadbee0 as *mut u8) = 42;
    };
}

#[allow(unconditional_recursion)]
fn _test_stack_overflow() {
    _test_stack_overflow(); // for each recursion, the return address is pushed
}

extern "x86-interrupt" fn page_fault_handler(stack_frame: idt::StackFrame, error_code: u64) {
    let error_code = PageFaultErrorCode(error_code);
    hal::interrupts::without_interrupts(|| {
        log!(RING_BUFFER, "PAGE FAULT {error_code:?}");
        log!(RING_BUFFER, "{stack_frame:#x?}");
    });
    hal::halt();
}

extern "x86-interrupt" fn gp_fault_handler(stack_frame: idt::StackFrame, error_code: u64) {
    hal::interrupts::without_interrupts(|| {
        log!(RING_BUFFER, "GENERAL PROTECTION FAULT {error_code:?}");
        log!(RING_BUFFER, "{stack_frame:#x?}");
    });
    hal::halt();
}

extern "x86-interrupt" fn double_fault_handler(stack_frame: idt::StackFrame, error_code: u64) -> ! {
    hal::interrupts::without_interrupts(|| {
        log!(RING_BUFFER, "DOUBLE FAULT {error_code:?}");
        log!(RING_BUFFER, "{stack_frame:#x?}");
    });
    loop {
        hal::halt();
    }
}

fn timer_handler(_stack_frame: idt::StackFrame) {
    let count = COUNTER.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
    DEBUG_WRITER
        .get()
        .unwrap()
        .lock()
        .write_fmt(format_args!("{},", count))
        .unwrap();

    unsafe {
        PICS.get()
            .unwrap()
            .lock()
            .notify_end_of_interrupt(TIMER_IRQ as u8);
    };
}

#[unsafe(naked)]
pub extern "C" fn syscall_entry() {
    core::arch::naked_asm!(
        "
            push rax
            push rdi
            push rsi
            push rdx
            push r10
            push r8
            push r9
            
            mov rdi, rsp
            call syscall_handler
            
            pop r9
            pop r8
            pop r10
            pop rdx
            pop rsi
            pop rdi
            pop rax
            
            iretq
            "
    )
}

#[derive(Debug)]
#[repr(C)]
pub struct SyscallFrame {
    pub r9: usize,
    pub r8: usize,
    pub r10: usize,
    pub rdx: usize,
    pub rsi: usize,
    pub rdi: usize,
    pub rax: usize,
}

#[unsafe(no_mangle)]
pub extern "C" fn syscall_handler(frame: &mut SyscallFrame) {
    let ret = match frame.rax {
        SYS_SWITCH => handle_sys_switch(frame.rdi),
        SYS_GET_PID => handle_sys_get_pid(),
        SYS_SPAWN => handle_sys_spawn(frame.rdi, frame.rsi),
        SYS_EXIT => handle_sys_exit(frame.rdi),
        SYS_WAIT => handle_sys_wait(frame.rdi),
        SYS_LOG => handle_sys_log(frame.rdi, frame.rsi),
        _ => unimplemented!("unhandled syscall {}", frame.rax),
    };

    frame.rax = ret;
}

fn shared_handler(irq_no: u64, stack_frame: idt::StackFrame) {
    match irq_no {
        TIMER_IRQ => timer_handler(stack_frame),
        val => unimplemented!("unimplemented interrupt {val:#x?}"),
    };
}
