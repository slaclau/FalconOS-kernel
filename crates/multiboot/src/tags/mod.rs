use core::fmt::Debug;
use core::ptr::{self, from_raw_parts, Pointee};

pub use crate::tags::header::TagHeader;

mod boot_loader_name;
pub use boot_loader_name::BootLoaderNameTag;

mod command_line;
pub use command_line::CommandLineTag;

mod elf_sections;
pub use elf_sections::ElfSectionsTag;

mod header;

mod memory_map;
pub use memory_map::{MemoryMapTag, MemoryMapTagEntryType};

mod modules;
pub use modules::ModuleTag;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
#[repr(u32)]
pub enum TagType {
    End = 0,
    CommandLine = 1,
    BootLoaderName = 2,
    Modules = 3,
    BasicMemoryInformation = 4,
    BiosBootDevice = 5,
    MemoryMap = 6,
    VbeInfo = 7,
    FrameBufferInfo = 8,
    ElfSymbols = 9,
    ApmTable = 10,
    Efi32BitSystemTablePointer = 11,
    Efi64BitSystemTablePointer = 12,
    SmbiosTables = 13,
    AcpiOldRsdp = 14,
    AcpiNewRsdp = 15,
    NetworkingInfo = 16,
    EfiMemoryMap = 17,
    EfiBootServicesNotTerminated = 18,
    Efi32BitImageHandlePointer = 19,
    Efi64BitImageHandlePointer = 20,
    ImageLoadBasePhysicalAddress = 21,
}

pub trait Tag: Debug {
    const TYPE: TagType;

    fn dst_len(header: &TagHeader) -> usize;
}

#[repr(C)]
pub struct TagContainer {
    pub header: TagHeader,
    pub(crate) payload: [u8],
}

impl<'a> TagContainer {
    pub fn from_bytes(bytes: &[u8]) -> &Self {
        unsafe { &*(bytes as *const _ as *const Self) }
    }

    pub fn cast<T: Tag + ?Sized>(&self) -> &T
    where
        T: Pointee<Metadata = usize>,
    {
        assert!(self.header.tag_type == T::TYPE);

        let base_ptr = ptr::addr_of!(self.header);

        let dst_len = T::dst_len(&self.header);

        let ts_ptr = from_raw_parts(base_ptr, dst_len);
        let ts_ref = unsafe { &*ts_ptr };
        ts_ref
    }
}
impl Debug for TagContainer {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("TagContainer")
            .field("header", &self.header)
            .finish()
    }
}
