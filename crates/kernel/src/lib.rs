#![no_std]
#![feature(abi_x86_interrupt)]
#![feature(debug_closure_helpers)]

use core::fmt::Write;

use hal::halt;
use spin::Mutex;

use crate::utils::ring_buffer::{RING_BUFFER_LENGTH, RingBuffer};

mod arch;
mod utils;

pub use arch::*;

pub static RING_BUFFER: Mutex<RingBuffer<RING_BUFFER_LENGTH>> =
    Mutex::new(RingBuffer::<RING_BUFFER_LENGTH>::new());

#[cfg(debug_assertions)]
mod debug;

pub fn kernel_main() -> ! {
    log!(RING_BUFFER, "kernel_main called");

    halt();
    loop {}
}

pub fn kernel_shared_init() {
    log!(RING_BUFFER, "kernel_shared_init called");
}
