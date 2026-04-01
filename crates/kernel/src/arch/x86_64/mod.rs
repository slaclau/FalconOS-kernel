use core::fmt::Write;

use spin::{Mutex, Once};

use crate::{RING_BUFFER, log};

mod interrupts;
mod memory;
pub use memory::{PhysicalAddress, VirtualAddress};
mod pic;
mod port;
mod qemu;
mod segmentation;
mod tables;

pub use qemu::QemuDebugWriter;

pub static DEBUG_WRITER: Once<Mutex<QemuDebugWriter>> = Once::new();

unsafe extern "C" {
    // static _ring_buffer_start: usize;
    // static _ring_buffer_end: usize;
}

#[cfg_attr(not(test), panic_handler)]
#[allow(unused)]
fn panic(info: &core::panic::PanicInfo) -> ! {
    log!(RING_BUFFER, "PANIC: {:?}", info);
    loop {
        hal::halt();
    }
}

#[cfg(all(target_arch = "x86_64"))]
#[unsafe(no_mangle)]
pub extern "C" fn kernel_start(mb_ptr: u32, mb_magic: u32) -> ! {
    use crate::{RING_BUFFER, kernel_main, kernel_shared_init, log};
    log!(
        RING_BUFFER,
        "kernel_start (x86_64) called with mb_magic={mb_magic:#x} and mb_ptr={mb_ptr:#x}"
    );

    DEBUG_WRITER.call_once(|| Mutex::new(QemuDebugWriter {}));

    kernel_shared_init(mb_ptr, mb_magic);

    interrupts::configure();

    kernel_main();
}
