use core::{fmt::Debug, iter::from_fn};

use crate::{Tag, TagHeader, TagType};

#[derive(Debug)]
#[repr(C)]
pub struct ElfSectionsTag {
    pub header: TagHeader,
    number: u32,
    entry_size: u32,
    pub shndx: u32,
    entry_bytes: [u8],
}

impl Tag for ElfSectionsTag {
    const TYPE: TagType = TagType::ElfSymbols;

    fn dst_len(header: &TagHeader) -> usize {
        header.size as usize - size_of::<TagHeader>() - 12
    }
}

impl ElfSectionsTag {
    pub fn entries(&self) -> impl Iterator<Item = elf::SectionHeader> {
        let mut count = 0;

        from_fn(move || {
            if count < self.number {
                let endianness = elf::Endianness::Little;
                let architecture = match self.entry_size {
                    40 => elf::Architecture::Bits32,
                    64 => elf::Architecture::Bits64,
                    val => panic!("This should not happen - size must be 40 or 64 not {val}"),
                };
                let ret = Some(elf::SectionHeader::from_bytes(
                    architecture,
                    endianness,
                    &self.entry_bytes[(count * self.entry_size) as usize
                        ..((count + 1) * self.entry_size) as usize],
                ));
                count += 1;
                ret
            } else {
                None
            }
        })
    }
}
