#![no_std]
#![no_main]

#[cfg_attr(not(test), panic_handler)]
#[allow(unused)]
fn panic(info: &core::panic::PanicInfo) -> ! {
    loop {
        hal::halt();
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    let t = "test";
    let mut i = 0;
    loop {
        i += 1;
        if i % 10000000 == 0 {
            syscall::log(&t);
        }
    }
}
