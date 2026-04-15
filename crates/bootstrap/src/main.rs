#![no_std]
#![no_main]

use hal::halt;

#[cfg_attr(not(test), panic_handler)]
#[allow(unused)]
fn panic(info: &core::panic::PanicInfo) -> ! {
    loop {
        hal::halt();
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    let (_res, msg) = syscall::recv(0);
    let bytes = msg.data.map(|word| word.to_be_bytes());
    let bytes = bytes.as_flattened();
    let msg = str::from_utf8(bytes).unwrap();

    syscall::log(msg);
    loop {
        halt();
    }
}
