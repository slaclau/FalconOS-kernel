use core::arch::asm;

use crate::arch::x86_64::tables::{PrivilegeLevel, TablePointer};

pub struct Table<const MAX: usize> {
    entries: [u64; MAX],
    length: usize,
}

impl<const MAX: usize> Table<MAX> {
    pub fn empty() -> Self {
        assert!(MAX > 0, "Table cannot be empty");
        assert!(
            MAX < (1 << 13),
            "Table cannot be larger than {} entries",
            1 << 13
        );

        Self {
            entries: [0; MAX],
            length: 1,
        }
    }

    fn push(&mut self, value: u64) -> usize {
        let index = self.length;
        self.entries[index] = value;
        self.length += 1;
        index
    }

    pub fn append(&mut self, entry: Descriptor) -> SegmentSelector {
        let index = match entry {
            Descriptor::UserSegment(value) => {
                assert!(self.length < MAX.saturating_sub(1), "Table is full");
                self.push(value)
            }
            Descriptor::SystemSegment(value_low, value_high) => {
                assert!(
                    self.length < MAX.saturating_sub(2),
                    "Table needs two free spaces for a SystemSegment"
                );
                let index = self.push(value_low);
                self.push(value_high);
                index
            }
        };

        SegmentSelector::new(index as u16, entry.get_dpl())
    }

    pub fn pointer(&self) -> TablePointer {
        TablePointer {
            offset: self as *const _ as u64,
            size: (self.length * size_of::<u64>()) as u16 - 1,
        }
    }

    pub unsafe fn load(&'static self) {
        let gdt = &self.pointer();
        unsafe {
            asm!("lgdt [{}]", in(reg) gdt, options(readonly, nostack, preserves_flags));
        }
    }
}

pub enum Descriptor {
    /// Descriptor for a code or data segment.
    ///
    /// Since segmentation is no longer supported in 64-bit mode, almost all of
    /// code and data descriptors is ignored. Only some flags are still used.
    UserSegment(u64),
    /// A system segment descriptor such as a LDT or TSS descriptor.
    SystemSegment(u64, u64),
}

impl Descriptor {
    pub fn get_dpl(self) -> PrivilegeLevel {
        let value_low = match self {
            Descriptor::UserSegment(v) => v,
            Descriptor::SystemSegment(v, _) => v,
        };
        let dpl = (value_low & (3 << 45)) >> 45;
        PrivilegeLevel::from_u16(dpl as u16)
    }

    /// Creates a segment descriptor for a 64-bit kernel code segment. Suitable
    /// for use with `syscall` or 64-bit `sysenter`.
    #[inline]
    pub const fn kernel_code_segment() -> Descriptor {
        Descriptor::UserSegment(0x00af9b000000ffff)
    }

    /// Creates a segment descriptor for a kernel data segment (32-bit or
    /// 64-bit). Suitable for use with `syscall` or `sysenter`.
    #[inline]
    pub const fn kernel_data_segment() -> Descriptor {
        Descriptor::UserSegment(0x00cf93000000ffff)
    }

    /// Creates a segment descriptor for a ring 3 data segment (32-bit or
    /// 64-bit). Suitable for use with `sysret` or `sysexit`.
    #[inline]
    pub const fn user_data_segment() -> Descriptor {
        Descriptor::UserSegment(0x00cff3000000ffff)
    }
}

pub struct SegmentSelector(u16);

impl SegmentSelector {
    pub fn new(index: u16, rpl: PrivilegeLevel) -> Self {
        Self(index << 3 | rpl as u16)
    }
}
