use core::fmt::Debug;

use crate::{Tag, TagHeader, TagType};

pub struct ModuleTag {
    start: u32,
    end: u32,
    string: [u8],
}

impl Tag for ModuleTag {
    const TYPE: TagType = TagType::Modules;

    fn dst_len(header: &TagHeader) -> usize {
        header.size as usize - size_of::<TagHeader>()
    }
}

impl ModuleTag {
    fn _string(&self) -> &str {
        str::from_utf8(&self.string[0..self.string.len() - 1]).expect("Could not parse Module")
    }
}

impl Debug for ModuleTag {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ModuleTag")
            .field("start", &self.start)
            .field("end", &self.end)
            .field("raw_string", &&self.string)
            // .field("string", &self.string())
            .finish()
    }
}
