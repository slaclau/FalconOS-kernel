#![allow(unused)]

use core::{arch::asm, cmp, fmt::Debug, mem};

use crate::{
    arch::x86_64::{
        segmentation::tss::TaskStateSegment,
        tables::{PrivilegeLevel, TablePointer},
    },
    log, utils::bits::{get_bit, get_bits},
};

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

    pub fn entries(&self) -> &[u64] {
        &self.entries[..self.length]
    }
}

#[derive(Clone, Copy)]
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
    pub fn get_bits(self) -> u128 {
        match self {
            Descriptor::UserSegment(v) => v as u128,
            Descriptor::SystemSegment(value_low, value_high) => {
                ((value_high as u128) << 64) + value_low as u128
            }
        }
    }
    pub fn get_dpl(self) -> PrivilegeLevel {
        let value_low = match self {
            Descriptor::UserSegment(v) => v,
            Descriptor::SystemSegment(v, _) => v,
        };
        let dpl = (value_low & (3 << 45)) >> 45;
        PrivilegeLevel::from_u16(dpl as u16)
    }
    pub fn get_access_byte(self) -> AccessByte {
        let v = match self {
            Descriptor::SystemSegment(v, _) => v,
            Descriptor::UserSegment(v) => v,
        };
        AccessByte(get_bits(v, 40, 48) as u8)
    }

    pub fn get_flags(self) -> DescriptorFlags {
        let v = match self {
            Descriptor::SystemSegment(v, _) => v,
            Descriptor::UserSegment(v) => v,
        };
        DescriptorFlags(get_bits(v, 52, 56) as u8)
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

    #[inline]
    pub fn tss_segment(tss: &'static TaskStateSegment) -> Descriptor {
        // SAFETY: The pointer is derived from a &'static reference, which ensures its validity.
        unsafe { Self::tss_segment_unchecked(tss) }
    }

    /// Similar to [`Descriptor::tss_segment`], but unsafe since it does not enforce a lifetime
    /// constraint on the provided TSS.
    ///
    /// # Safety
    /// The caller must ensure that the passed pointer is valid for as long as the descriptor is
    /// being used.
    #[inline]
    pub unsafe fn tss_segment_unchecked(tss: *const TaskStateSegment) -> Descriptor {
        // SAFETY: if iomap_size is zero, there are no requirements to uphold.
        unsafe { Self::tss_segment_raw(tss, 0) }
    }

    /// Creates a TSS system descriptor for the given TSS, setting up the IO permissions bitmap.
    ///
    /// # Safety
    ///
    /// There must be a valid IO map at `(tss as *const u8).offset(tss.iomap_base)`
    /// of length `iomap_size`, with the terminating `0xFF` byte. Additionally, `iomap_base` must
    /// not exceed `0xDFFF`.
    unsafe fn tss_segment_raw(tss: *const TaskStateSegment, iomap_size: u16) -> Descriptor {
        let ptr = tss as u64;

        let mut low = 0;
        // base
        low |= (get_bits(ptr, 0, 24) << 16);
        low |= (get_bits(ptr, 24, 32) << 56);
        // limit (the `-1` is needed since the bound is inclusive)
        let iomap_limit = u64::from(unsafe { (*tss).iopb }) + u64::from(iomap_size);
        low |=
            ((cmp::max(mem::size_of::<TaskStateSegment>() as u64, iomap_limit) - 1) as u16) as u64;

        // type (0b1001 = available 64-bit tss)
        low |= (0b1001 << 40);
        low |= (0b1 << 47);

        let mut high = 0;
        high |= (ptr << 32);

        Descriptor::SystemSegment(low, high)
    }
}

impl Debug for Descriptor {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let name = match self {
            Descriptor::SystemSegment(value_low, value_high) => "Descriptor::SystemSegment",
            Descriptor::UserSegment(value) => "Descriptor::UserSegment",
        };
        let base = match self {
            Descriptor::SystemSegment(value_low, value_high) => {
                get_bits(*value_low, 16, 32)
                    | (get_bits(*value_low, 32, 40) << 8)
                    | (get_bits(*value_low, 56, 64) << 16)
                    | (get_bits(*value_high, 0, 32) << 32)
            }
            Descriptor::UserSegment(value) => {
                get_bits(*value, 16, 32)
                    | (get_bits(*value, 32, 40) << 8)
                    | (get_bits(*value, 56, 64) << 16)
            }
        };
        let limit = match self {
            Descriptor::SystemSegment(v, _) => get_bits(*v, 0, 16) | (get_bits(*v, 48, 52) << 16),
            Descriptor::UserSegment(v) => get_bits(*v, 0, 16) | (get_bits(*v, 48, 52) << 16),
        };
        f.debug_struct(name)
            // .field_with("bits", |f| {
            //     f.write_fmt(format_args!("{:#018x}", self.get_bits()))
            // })
            .field_with("base", |f| f.write_fmt(format_args!("{:#018x}", &base)))
            .field_with("limit", |f| f.write_fmt(format_args!("{:#08x}", &limit)))
            .field("dpl", &self.get_dpl())
            .finish()
    }
}

pub struct AccessByte(u8);

impl AccessByte {
    fn get_present(&self) -> bool {
        get_bit(self.0 as u64, 7) > 0
    }
    fn get_dpl(&self) -> PrivilegeLevel {
        PrivilegeLevel::from_u16(get_bits(self.0 as u64, 5, 7) as u16)
    }
    fn get_is_system(&self) -> bool {
        get_bit(self.0 as u64, 4) == 0
    }
    fn get_is_executable(&self) -> bool {
        if self.get_is_system() {
            panic!("Not implemented for System Descriptors")
        };
        get_bit(self.0 as u64, 3) > 0
    }
    fn get_dc(&self) -> bool {
        if self.get_is_system() {
            panic!("Not implemented for System Descriptors")
        };
        get_bit(self.0 as u64, 2) > 0
    }
    fn get_rw(&self) -> bool {
        if self.get_is_system() {
            panic!("Not implemented for System Descriptors")
        };
        get_bit(self.0 as u64, 1) > 0
    }
    fn get_accessed(&self) -> bool {
        if self.get_is_system() {
            panic!("Not implemented for System Descriptors")
        };
        get_bit(self.0 as u64, 0) > 0
    }
    fn get_system_segment_type(&self) -> SystemSegmentType {
        if !self.get_is_system() {
            panic!("Not implemented for Code/Data Descriptors")
        };
        match get_bits(self.0 as u64, 0, 4) {
            0x2 => SystemSegmentType::LDT,
            0x9 => SystemSegmentType::AvailableTSS,
            0xB => SystemSegmentType::BusyTSS,
            v => panic!("{v:#x} is not a valid type"),
        }
    }
}

impl Debug for AccessByte {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if self.get_is_system() {
            f.debug_struct("AccessByte")
                .field("present", &self.get_present())
                .field("dpl", &self.get_dpl())
                .field("type", &self.get_system_segment_type())
                .finish()
        } else {
            unimplemented!()
        }
    }
}

#[derive(Debug)]
enum SystemSegmentType {
    LDT,
    AvailableTSS,
    BusyTSS,
}
pub struct DescriptorFlags(u8);

impl DescriptorFlags {}

impl Debug for DescriptorFlags {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("DescriptorFlags").finish()
    }
}

#[derive(Clone, Copy)]
pub struct SegmentSelector(u16);

impl SegmentSelector {
    pub fn new(index: u16, rpl: PrivilegeLevel) -> Self {
        Self(index << 3 | rpl as u16)
    }

    pub fn as_u64(&self) -> u64 {
        self.0 as u64
    }

    pub fn get_rpl(&self) -> PrivilegeLevel {
        PrivilegeLevel::from_u16(self.0 << 14 >> 14)
    }

    pub fn get_ti(&self) -> bool {
        (self.0 & 0b0100) != 0
    }

    pub fn get_index(&self) -> u16 {
        self.0 >> 3
    }
}

impl Debug for SegmentSelector {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("SegmentSelector")
            .field_with("bits", |f| f.write_fmt(format_args!("{:#018b}", self.0)))
            .field("rpl", &self.get_rpl())
            .field("ti", &self.get_ti())
            .field("index", &mut self.get_index())
            .finish()
    }
}

#[inline]
pub unsafe fn load_tss(sel: SegmentSelector) {
    unsafe {
        asm!("ltr {0:x}", in(reg) sel.0, options(nostack, preserves_flags));
    }
}
