use core::fmt::Write;

use hal;
use spin::{Mutex, Once};

use crate::{
    RING_BUFFER,
    arch::x86_64::{
        pic::ChainedPics,
        tables::{
            gdt,
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
)> = Once::new();

static PICS: Once<Mutex<ChainedPics>> = Once::new();

const TIMER_INTERRUPT: usize = 0;
static TIMER_WRITER: Once<Mutex<Writer>> = Once::new(); // TODO: This will have a proper implementation to trigger the scheduler

pub fn configure() {
    log!(RING_BUFFER, "interrupts::configure called");

    create_and_load_gdt();
    create_and_load_idt();

    // enable_timer();

    // _test_stack_overflow();
}

fn create_and_load_gdt() {
    GDT.call_once(|| {
        let mut gdt = gdt::Table::empty();
        let kcode_selector = gdt.append(gdt::Descriptor::kernel_code_segment());
        let kdata_selector = gdt.append(gdt::Descriptor::kernel_data_segment());
        let udata_selector = gdt.append(gdt::Descriptor::user_data_segment());

        (gdt, kcode_selector, kdata_selector, udata_selector)
    });

    unsafe { GDT.get().unwrap().0.load() };
    let pointer = GDT.get().unwrap().0.pointer();
    let off = pointer.offset;
    log!(
        RING_BUFFER,
        "GDT Descriptor size {:#x} offset {:#x}",
        pointer.size,
        off
    );

    log!(RING_BUFFER, "loaded GDT");
}

fn create_and_load_idt() {
    IDT.call_once(|| {
        let mut idt = idt::Table::new();

        unsafe {
            idt.page_pault().options.set_present(true);
            idt.page_pault()
                .set_handler_addr(page_fault_handler as *const () as u64);
            idt.double_fault().options.set_present(true);
            idt.double_fault()
                .set_handler_addr(double_fault_handler as *const () as u64);
            idt.interrupts()[TIMER_INTERRUPT].options.set_present(true);
            idt.interrupts()[TIMER_INTERRUPT].set_handler_addr(timer_handler as *const () as u64);
        };

        idt
    });

    unsafe { IDT.get().unwrap().load() };
    let pointer = IDT.get().unwrap().pointer();
    let off = pointer.offset;
    log!(
        RING_BUFFER,
        "IDT Descriptor size {:#x} offset {:#x}",
        pointer.size,
        off
    );

    log!(RING_BUFFER, "loaded IDT");
}

fn enable_timer() {
    TIMER_WRITER.call_once(|| Mutex::new(make_writer(0xb8000)));
    PICS.call_once(|| {
        let mut pics = unsafe { ChainedPics::new(32, 40) };
        unsafe { pics.initialize() };
        Mutex::new(pics)
    });
    log!(RING_BUFFER, "enabled timer (PIT)");

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
        RING_BUFFER.lock().dump(make_writer(0xb8000));
    });
    hal::halt();
}

extern "x86-interrupt" fn double_fault_handler(stack_frame: idt::StackFrame, error_code: u64) -> ! {
    hal::interrupts::without_interrupts(|| {
        log!(RING_BUFFER, "DOUBLE FAULT {error_code:?}");
        log!(RING_BUFFER, "{stack_frame:?}");
        RING_BUFFER.lock().dump(make_writer(0xb8000));
    });
    loop {
        hal::halt();
    }
}

extern "x86-interrupt" fn timer_handler(_stack_frame: idt::StackFrame) {
    hal::interrupts::without_interrupts(|| TIMER_WRITER.get().unwrap().lock().write_string("."));
    unsafe {
        PICS.get().unwrap().lock().notify_end_of_interrupt(32);
    };
}
