use core::fmt::Debug;

use crate::{Tag, TagHeader, TagType};

#[repr(C)]
pub struct CommandLineTag {
    header: TagHeader,
    string: [u8],
}

impl Tag for CommandLineTag {
    const TYPE: TagType = TagType::CommandLine;

    fn dst_len(header: &TagHeader) -> usize {
        header.size as usize - size_of::<TagHeader>()
    }
}

impl CommandLineTag {
    fn string(&self) -> &str {
        str::from_utf8(&self.string[0..self.string.len() - 1]).expect("Could not parse CommandLine")
    }
}

impl Debug for CommandLineTag {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("CommandLineTag")
            .field("header", &self.header)
            .field("string", &self.string())
            .finish()
    }
}