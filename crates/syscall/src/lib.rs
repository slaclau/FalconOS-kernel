#![no_std]
#![feature(const_trait_impl)]
#![feature(const_ops)]

use core::{
    fmt::Debug,
    ops::{BitAnd, BitOr},
};

use arch::*;
pub use number::*;

use crate::{ipc::IpcError, process::ProcessError};

pub mod _log;
mod arch;
pub mod cap;
pub mod ipc;
mod number;
pub mod process;

pub struct SyscallReturnWords {
    words: [usize; 6],
}
pub struct SyscallOut {
    pub ret: usize,
    pub words: SyscallReturnWords,
}

impl SyscallOut {
    fn new(ret: usize, words: [usize; 6]) -> Self {
        Self {
            ret,
            words: SyscallReturnWords { words },
        }
    }
}

impl From<SyscallOut> for Result<[usize; 6], SyscallError> {
    fn from(value: SyscallOut) -> Self {
        if value.ret == 0 {
            Ok(value.words.words)
        } else {
            Err(unsafe { core::mem::transmute::<usize, SyscallError>(value.ret) })
        }
    }
}

pub type SyscallResult<T> = Result<T, SyscallError>;

pub fn log(message: &str) -> Result<(), SyscallError> {
    unsafe { syscall2(SYS_LOG, message.as_ptr() as usize, message.len()) }.map(|_| ())
}

#[derive(Debug)]
#[repr(u16)]
pub enum SyscallError {
    Ok = 0,
    RightsError(RightsError),
    IpcError(IpcError),
    ProcessError(ProcessError),
    InvalidObject,
    NoCap,
    Unknown,
}

#[derive(Debug)]
#[repr(C)]
pub enum RightsError {
    NoRead,
    NoWrite,
    NoExec,
    NoGrant,
    Unknown,
}
