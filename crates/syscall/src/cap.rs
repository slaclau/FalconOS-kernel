use core::{marker::PhantomData, mem};

use crate::{process::Process, *};

pub struct Untyped;

pub type CapHandle = usize;
pub trait CapType {}

#[derive(Clone, Copy, Debug)]
pub struct Cap<T: CapType> {
    pub handle: CapHandle,
    phantom: PhantomData<T>,
}

impl<T: CapType + Debug> Cap<T> {
    pub(crate) fn new(handle: CapHandle) -> Self {
        Self {
            handle,
            phantom: PhantomData,
        }
    }

    pub fn derive(self, mask: Rights) -> SyscallResult<Self> {
        let handle = derive_cap(self.handle, mask)?;
        Ok(Self {
            handle,
            phantom: PhantomData,
        })
    }

    pub fn r#move(self, process: Cap<Process>) -> SyscallResult<CapHandle> {
        format_log!("move cap {self:?} to {process:?}");
        move_cap(process.handle, self.handle)
    }

    #[allow(clippy::missing_safety_doc)]
    pub unsafe fn from_handle(handle: CapHandle) -> Self {
        Self {
            handle,
            phantom: PhantomData,
        }
    }
}

#[derive(Clone, Copy)]
pub struct Rights(u8);

impl Rights {
    pub const ALL: Rights = Rights(u8::MAX);
    pub const READ: Rights = Rights(0x1);
    pub const WRITE: Rights = Rights(0x2);
    pub const EXEC: Rights = Rights(0x4);
    pub const GRANT: Rights = Rights(0x8);
    pub const RWE: Rights = Rights::READ | Rights::WRITE | Rights::EXEC;

    pub fn read(self) -> bool {
        (self & Self::READ).0 > 0
    }

    pub fn write(self) -> bool {
        (self & Self::WRITE).0 > 0
    }

    pub fn exec(self) -> bool {
        (self & Self::EXEC).0 > 0
    }

    pub fn grant(self) -> bool {
        (self & Self::GRANT).0 > 0
    }

    pub fn matches(self, mask: Rights) -> bool {
        (self & mask).0 > 0
    }
}

impl Debug for Rights {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Rights")
            .field("R", &self.read())
            .field("W", &self.write())
            .field("X", &self.exec())
            .field("G", &self.grant())
            .finish()
    }
}

impl const BitAnd for Rights {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl const BitOr for Rights {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl From<usize> for Rights {
    fn from(value: usize) -> Self {
        assert!(value <= u8::MAX as usize);
        Self(value as u8)
    }
}

fn derive_cap(cap_id: CapHandle, mask: Rights) -> SyscallResult<CapHandle> {
    unsafe { syscall2(SYS_DERIVE_CAP, cap_id, mask.0 as usize) }.map(|words| words[0])
}

fn move_cap(process_handle: CapHandle, cap_handle: CapHandle) -> SyscallResult<CapHandle> {
    unsafe { syscall2(SYS_MOVE_CAP, process_handle, cap_handle) }.map(|words| words[0])
}

#[allow(clippy::enum_clike_unportable_variant)]
#[repr(usize)]
pub enum CapError {
    Ok = 0,
    NoGrant = 1,
    Unknown = usize::MAX,
}

impl From<usize> for CapError {
    fn from(value: usize) -> Self {
        if (0..=1).contains(&value) {
            unsafe { mem::transmute::<usize, CapError>(value) }
        } else {
            CapError::Unknown
        }
    }
}
