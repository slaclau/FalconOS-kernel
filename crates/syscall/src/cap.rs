use crate::{process::ProcessId, *};

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

pub fn derive_cap(cap_id: usize, mask: Rights) -> usize {
    unsafe { syscall2(SYS_DERIVE_CAP, cap_id, mask.0 as usize) }
}

pub fn move_cap(process_id: ProcessId, cap_id: usize) -> usize {
    unsafe { syscall2(SYS_MOVE_CAP, process_id, cap_id) }
}
