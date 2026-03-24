use core::fmt::Write;

mod interrupts;
mod pic;
mod port;
mod tables;

unsafe extern "C" {
    static _ring_buffer_start: usize;
    static _ring_buffer_end: usize;
}

#[cfg_attr(not(test), panic_handler)]
#[allow(unused)]
fn panic(info: &core::panic::PanicInfo) -> ! {
    loop {
        hal::halt();
    }
}

#[cfg(all(target_arch = "x86_64"))]
#[unsafe(no_mangle)]
pub extern "C" fn kernel_start(mb_ptr: u32, mb_magic: u32) {
    use crate::{RING_BUFFER, kernel_main, kernel_shared_init, log};

    kernel_shared_init();
    log!(
        RING_BUFFER,
        "kernel_start called with mb_magic={mb_magic:#x} and mb_ptr={mb_ptr:#x}"
    );

    interrupts::configure();

    kernel_main();
}
