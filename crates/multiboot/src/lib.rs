#![no_std]
#![feature(ptr_metadata)]
#![feature(debug_closure_helpers)]

use core::{fmt::Debug, iter, ptr::Pointee, slice};

mod tags;
pub use tags::*;

pub const MULTIBOOT2_MAGIC: u32 = 0x36D76289;

#[derive(Debug)]
pub struct BootInformationHeader {
    pub length: u32,
    _reserved: u32,
}

pub struct BootInformation<'a> {
    header: &'a BootInformationHeader,
    pub payload: &'a [u8],
}

impl<'a> BootInformation<'a> {
    #[allow(clippy::missing_safety_doc)]
    pub unsafe fn load(ptr: u32) -> Self {
        let header = unsafe { &*(ptr as *const BootInformationHeader) };
        let payload = unsafe {
            slice::from_raw_parts(
                (ptr + size_of::<BootInformationHeader>() as u32) as *const u8,
                header.length as usize - size_of::<BootInformationHeader>(),
            )
        };

        Self { header, payload }
    }

    pub fn get_tags(&self) -> impl Iterator<Item = &TagContainer> {
        let mut i = 0;
        iter::from_fn(move || {
            if i < self.payload.len() {
                let header = TagHeader::from_bytes(
                    self.payload[i..i + size_of::<TagHeader>()]
                        .try_into()
                        .expect("could not create header"),
                );
                let tag = TagContainer::from_bytes(
                    self.payload[i..i + header.size as usize]
                        .try_into()
                        .expect("could not create container"),
                );
                let bytes = header.size as usize;
                let rounded_bytes = bytes.div_ceil(8) * 8;
                i += rounded_bytes;
                Some(tag)
            } else {
                None
            }
        })
    }

    pub fn command_line(&self) -> &CommandLineTag {
        self.get_tag::<CommandLineTag>()
    }

    pub fn boot_loader_name(&self) -> &BootLoaderNameTag {
        self.get_tag::<BootLoaderNameTag>()
    }

    pub fn memory_map(&self) -> &MemoryMapTag {
        self.get_tag::<MemoryMapTag>()
    }

    pub fn get_tag<T: Tag + ?Sized + Pointee<Metadata = usize>>(&self) -> &T {
        self.get_tags()
            .find(|t| t.header.tag_type == T::TYPE)
            .map(|t| t.cast())
            .expect("Could not find a matching tag")
    }
}

impl<'a> Debug for BootInformation<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("BootInformation")
            .field("header", &self.header)
            .finish()
    }
}
