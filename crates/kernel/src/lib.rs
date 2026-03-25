#![no_std]
#![feature(abi_x86_interrupt)]
#![feature(debug_closure_helpers)]
#![allow(unused)]

use core::fmt::Write;

use hal::halt;
use spin::Mutex;

use crate::{
    debug::make_writer,
    utils::ring_buffer::{RING_BUFFER_LENGTH, RingBuffer},
};

mod arch;
mod utils;

pub static RING_BUFFER: Mutex<RingBuffer<RING_BUFFER_LENGTH>> =
    Mutex::new(RingBuffer::<RING_BUFFER_LENGTH>::new());

#[cfg(debug_assertions)]
mod debug;

pub fn kernel_main() -> ! {
    log!(RING_BUFFER, "kernel_main called");

    RING_BUFFER
        .lock()
        .dump_with_reason("At end", make_writer(0xb8000));

    halt();
    loop {}
}

pub fn kernel_shared_init() {
    log!(RING_BUFFER, "kernel_shared_init called");
}
