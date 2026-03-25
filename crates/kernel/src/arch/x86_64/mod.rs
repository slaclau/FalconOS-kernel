use core::fmt::Write;

use spin::{Mutex, MutexGuard, Once};

use crate::{RING_BUFFER, debug::{Writer, make_writer}, log};

mod interrupts;
mod pic;
mod port;
mod segmentation;
mod tables;

pub static DEBUG_WRITER: Once<Mutex<Writer>> = Once::new();

pub struct WriterWrapper<'a>(pub MutexGuard<'a, Writer>);

impl Write for WriterWrapper<'_> {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.0.write_str(s)
    }
}

unsafe extern "C" {
    static _ring_buffer_start: usize;
    static _ring_buffer_end: usize;
}

#[cfg_attr(not(test), panic_handler)]
#[allow(unused)]
fn panic(info: &core::panic::PanicInfo) -> ! {
    log!(RING_BUFFER, "{:?}", info);
    RING_BUFFER.lock().dump_with_reason("PANIC", make_writer(0xb8000));
    loop {
        hal::halt();
    }
}

#[cfg(all(target_arch = "x86_64"))]
#[unsafe(no_mangle)]
pub extern "C" fn kernel_start(mb_ptr: u32, mb_magic: u32) -> ! {
    use crate::{RING_BUFFER, kernel_main, kernel_shared_init, log};

    DEBUG_WRITER.call_once(|| Mutex::new(make_writer(0xb8000)));


    kernel_shared_init();
    log!(
        RING_BUFFER,
        "kernel_start called with mb_magic={mb_magic:#x} and mb_ptr={mb_ptr:#x}"
    );

    interrupts::configure();

    kernel_main();
}
