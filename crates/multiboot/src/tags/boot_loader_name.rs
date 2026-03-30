use core::fmt::Debug;

use crate::{Tag, TagHeader, TagType};

#[repr(C)]
pub struct BootLoaderNameTag {
    header: TagHeader,
    string: [u8],
}
impl Tag for BootLoaderNameTag {
    const TYPE: TagType = TagType::BootLoaderName;

    fn dst_len(header: &TagHeader) -> usize {
        header.size as usize - size_of::<TagHeader>()
    }
}

impl BootLoaderNameTag {
    fn string(&self) -> &str {
        str::from_utf8(&self.string[0..self.string.len() - 1])
            .expect("Could not parse BootLoaderName")
            .trim_matches(char::from(0))
    }
}
impl Debug for BootLoaderNameTag {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("BootLoaderNameTag")
            .field("header", &self.header)
            .field("string", &self.string())
            .finish()
    }
}
