use core::fmt::Debug;

use crate::{Tag, TagHeader, TagType};

#[derive(Debug)]
#[repr(C)]
pub struct MemoryMapTag {
    header: TagHeader,
    entry_size: u32,
    entry_version: u32,
    pub entries: [MemoryMapTagEntry],
}

impl Tag for MemoryMapTag {
    const TYPE: TagType = TagType::MemoryMap;

    fn dst_len(header: &TagHeader) -> usize {
        (header.size as usize - size_of::<TagHeader>() - 8) / size_of::<MemoryMapTagEntry>()
    }
}

#[repr(C)]
pub struct MemoryMapTagEntry {
    pub base_addr: u64,
    pub length: u64,
    pub memory_area_type: MemoryMapTagEntryTypeId,
    _reserved: u32,
}

impl Debug for MemoryMapTagEntry {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("MemoryMapTagEntry")
            .field_with("base_addr", |f| {
                f.write_fmt(format_args!("{:#x}", &self.base_addr))
            })
            .field_with("length", |f| {
                f.write_fmt(format_args!("{:#x}", &self.length))
            })
            .field("memory_area_type", &self.memory_area_type)
            .finish()
    }
}

#[derive(Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum MemoryMapTagEntryType {
    Available = 1,
    Reserved = 2,
    AcpiAvailable = 3,
    ReservedHibernate = 4,
    Defective = 5,
    Custom(u32),
}

#[derive(PartialEq, Eq)]
pub struct MemoryMapTagEntryTypeId(u32);

impl From<&MemoryMapTagEntryTypeId> for MemoryMapTagEntryType {
    fn from(value: &MemoryMapTagEntryTypeId) -> Self {
        match value {
            MemoryMapTagEntryTypeId(1) => Self::Available,
            MemoryMapTagEntryTypeId(2) => Self::Reserved,
            MemoryMapTagEntryTypeId(3) => Self::AcpiAvailable,
            MemoryMapTagEntryTypeId(4) => Self::ReservedHibernate,
            MemoryMapTagEntryTypeId(5) => Self::Defective,
            MemoryMapTagEntryTypeId(val) => Self::Custom(*val),
        }
    }
}

impl From<&MemoryMapTagEntryType> for MemoryMapTagEntryTypeId {
    fn from(value: &MemoryMapTagEntryType) -> Self {
        match value {
            MemoryMapTagEntryType::Available => MemoryMapTagEntryTypeId(1),
            MemoryMapTagEntryType::Reserved => MemoryMapTagEntryTypeId(2),
            MemoryMapTagEntryType::AcpiAvailable => MemoryMapTagEntryTypeId(3),
            MemoryMapTagEntryType::ReservedHibernate => MemoryMapTagEntryTypeId(4),
            MemoryMapTagEntryType::Defective => MemoryMapTagEntryTypeId(5),
            MemoryMapTagEntryType::Custom(val) => MemoryMapTagEntryTypeId(*val),
        }
    }
}

impl PartialEq<MemoryMapTagEntryType> for MemoryMapTagEntryTypeId {
    fn eq(&self, other: &MemoryMapTagEntryType) -> bool {
        let val: Self = other.into();
        let val: u32 = val.0;
        self.0.eq(&val)
    }
}

impl PartialEq<MemoryMapTagEntryTypeId> for MemoryMapTagEntryType {
    fn eq(&self, other: &MemoryMapTagEntryTypeId) -> bool {
        let val: MemoryMapTagEntryTypeId = self.into();
        let val: u32 = val.0;
        other.0.eq(&val)
    }
}

impl Debug for MemoryMapTagEntryTypeId {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let from = MemoryMapTagEntryType::from(self);
        f.write_fmt(format_args!("{from:?}"))
    }
}
