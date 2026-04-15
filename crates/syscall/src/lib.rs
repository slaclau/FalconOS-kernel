#![no_std]
#![feature(const_trait_impl)]
#![feature(const_ops)]

use core::{
    fmt::Debug,
    ops::{BitAnd, BitOr},
};

use arch::*;
pub use number::*;

mod arch;
pub mod cap;
pub mod ipc;
mod number;
pub mod process;

pub fn log(message: &str) -> usize {
    unsafe { syscall2(SYS_LOG, message.as_ptr() as usize, message.len()) }
}
