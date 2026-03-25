use core::{arch::asm, fmt::Write};

use hal::{self, interrupts::without_interrupts};
use macros::make_handlers;
use spin::{Mutex, Once};

use crate::{
    RING_BUFFER,
    arch::x86_64::{
        pic::ChainedPics,
        segmentation::{self, tss},
        tables::{
            gdt::{self, load_tss},
            idt::{self, PageFaultErrorCode},
        },
    },
    debug::{Writer, make_writer},
    log,
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
const TIMER_INTERRUPT: usize = 0;
static TIMER_WRITER: Once<Mutex<Writer>> = Once::new(); // TODO: This will have a proper implementation to trigger the scheduler

struct Count(u64);
impl Count {
    pub fn increment(&mut self) {
        self.0 += 1;
    }
}
static COUNTER: Once<Mutex<Count>> = Once::new();

make_handlers!();

pub fn configure() {
    log!(RING_BUFFER, "interrupts::configure called");

    create_and_load_gdt();
    create_and_load_idt();

    enable_timer();

    RING_BUFFER
        .lock()
        .dump_with_reason("After loading", make_writer(0xb8000));
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
        let tss_selector = gdt.append(gdt::Descriptor::tss_segment(&TSS.get().unwrap()));

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
    log!(RING_BUFFER, "Loaded TSS");
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
            idt.interrupts()[TIMER_INTERRUPT]
                .set_handler_addr(irq_handler_0 as *const () as u64)
                .options
                .set_present(true);
        };

        idt
    });

    unsafe { IDT.get().unwrap().load() };

    log!(RING_BUFFER, "loaded IDT");
}

fn enable_timer() {
    COUNTER.call_once(|| Mutex::new(Count(0)));
    TIMER_WRITER.call_once(|| Mutex::new(make_writer(0xb8000)));
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
        log!(RING_BUFFER, "{stack_frame:?}");
        RING_BUFFER
            .lock()
            .dump_with_reason("PAGE FAULT", make_writer(0xb8000));
    });
    hal::halt();
}

extern "x86-interrupt" fn gp_fault_handler(stack_frame: idt::StackFrame, error_code: u64) {
    hal::interrupts::without_interrupts(|| {
        log!(RING_BUFFER, "GENERAL PROTECTION FAULT {error_code:?}");
        log!(RING_BUFFER, "{stack_frame:?}");
        RING_BUFFER
            .lock()
            .dump_with_reason("GENERAL PROTECTION FAULT", make_writer(0xb8000));
    });
    hal::halt();
}

extern "x86-interrupt" fn double_fault_handler(stack_frame: idt::StackFrame, error_code: u64) -> ! {
    hal::interrupts::without_interrupts(|| {
        log!(RING_BUFFER, "DOUBLE FAULT {error_code:?}");
        log!(RING_BUFFER, "{stack_frame:?}");
        RING_BUFFER
            .lock()
            .dump_with_reason("DOUBLE FAULT", make_writer(0xb8000));
    });
    loop {
        hal::halt();
    }
}

fn timer_handler(_stack_frame: idt::StackFrame) {
    COUNTER.get().unwrap().lock().increment();
    TIMER_WRITER
        .get()
        .unwrap()
        .lock()
        .write_fmt(format_args!("{},", COUNTER.get().unwrap().lock().0));
    unsafe {
        PICS.get().unwrap().lock().notify_end_of_interrupt(32);
    };
}

fn shared_handler(irq_no: u64, stack_frame: idt::StackFrame) {
    without_interrupts(|| {
        match irq_no {
            0 => timer_handler(stack_frame),
            _ => unimplemented!(),
        };
    })
}
