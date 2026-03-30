use core::fmt::Debug;

use crate::{Tag, TagHeader, TagType};

pub struct ModuleTag {
    header: TagHeader,
    pub start: u32,
    pub end: u32,
    string: [u8],
}

impl Tag for ModuleTag {
    const TYPE: TagType = TagType::Modules;

    fn dst_len(header: &TagHeader) -> usize {
        header.size as usize - size_of::<TagHeader>() - 8
    }
}

impl ModuleTag {
    fn string(&self) -> &str {
        str::from_utf8(&self.string[0..self.string.len() - 1])
            .expect("Could not parse Module")
            .trim_matches(char::from(0))
    }
}

impl Debug for ModuleTag {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ModuleTag")
            .field("header", &self.header)
            .field("start", &self.start)
            .field("end", &self.end)
            .field("string", &self.string())
            .finish()
    }
}
