#![no_std]

use core::{fmt::Debug, iter::from_fn, sync::atomic::AtomicUsize};

#[repr(C)]
pub struct Elf<'a>(pub &'a [u8]);

impl<'a> Elf<'a> {
    pub fn header(&self) -> Header {
        Header::from_bytes(self.0).expect("Could not parse the ELF header")
    }

    pub fn program_header(&self) -> ProgramHeaderTable<'_> {
        let header = self.header();

        ProgramHeaderTable {
            class: header.class,
            endianness: header.endianness,
            entry_size: header.ph_entry_size,
            entry_num: header.ph_entry_num,
            bytes: &self.0[header.phoff as usize
                ..(header.phoff + header.ph_entry_num as u64 * header.ph_entry_size as u64)
                    as usize],
        }
    }

    pub fn section_header(&self) -> SectionHeaderTable<'_> {
        let header = self.header();
        SectionHeaderTable {
            class: header.class,
            endianness: header.endianness,
            entry_size: header.sh_entry_size,
            entry_num: header.sh_entry_num,
            bytes: &self.0[header.shoff as usize
                ..(header.shoff + header.sh_entry_num as u64 * header.sh_entry_size as u64)
                    as usize],
        }
    }
}

const MAGIC: [u8; 4] = [0x7F, 0x45, 0x4C, 0x46];

#[derive(Debug)]
#[repr(C)]
pub struct Header {
    magic: [u8; 4],
    class: Architecture,
    endianness: Endianness,
    ei_version: u8,
    abi: u8,
    abi_version: u8,
    _padding: [u8; 7],
    pub file_type: FileType,
    isa: u16,
    version: u32,
    entry: u64,
    pub phoff: u64,
    pub shoff: u64,
    flags: u32,
    header_size: u16,
    pub ph_entry_size: u16,
    pub ph_entry_num: u16,
    pub sh_entry_size: u16,
    pub sh_entry_num: u16,
    shstrndx: u16,
}

impl Header {
    fn from_bytes(bytes: &[u8]) -> Result<Self, &str> {
        let magic = *bytes[0..4].as_array().expect("there are not enough bytes");
        if magic != MAGIC {
            return Err("invalid magic number");
        }

        let class = Architecture::from(bytes[4]);
        let endianness = Endianness::from(bytes[5]);

        let mut pos = AtomicUsize::new(6);

        let address_size = match class {
            Architecture::Bits32 => 4,
            Architecture::Bits64 => 8,
        };

        let get_u8 = || -> u8 {
            let ret = bytes[pos.load(core::sync::atomic::Ordering::Relaxed)];
            pos.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
            ret
        };
        let get_u16 = || -> u16 {
            let ret = match endianness {
                Endianness::Big => u16::from_be_bytes(
                    *bytes[pos.load(core::sync::atomic::Ordering::Relaxed)
                        ..pos.load(core::sync::atomic::Ordering::Relaxed) + 2]
                        .as_array()
                        .expect("Not enough bytes"),
                ),
                Endianness::Little => u16::from_le_bytes(
                    *bytes[pos.load(core::sync::atomic::Ordering::Relaxed)
                        ..pos.load(core::sync::atomic::Ordering::Relaxed) + 2]
                        .as_array()
                        .expect("Not enough bytes"),
                ),
            };
            pos.fetch_add(2, core::sync::atomic::Ordering::Relaxed);
            ret
        };
        let get_u32 = || -> u32 {
            let ret = match endianness {
                Endianness::Big => u32::from_be_bytes(
                    *bytes[pos.load(core::sync::atomic::Ordering::Relaxed)
                        ..pos.load(core::sync::atomic::Ordering::Relaxed) + 4]
                        .as_array()
                        .expect("Not enough bytes"),
                ),
                Endianness::Little => u32::from_le_bytes(
                    *bytes[pos.load(core::sync::atomic::Ordering::Relaxed)
                        ..pos.load(core::sync::atomic::Ordering::Relaxed) + 4]
                        .as_array()
                        .expect("Not enough bytes"),
                ),
            };
            pos.fetch_add(4, core::sync::atomic::Ordering::Relaxed);
            ret
        };
        let get_u64 = || -> u64 {
            let ret = match endianness {
                Endianness::Big => match class {
                    Architecture::Bits32 => u32::from_be_bytes(
                        *bytes[pos.load(core::sync::atomic::Ordering::Relaxed)
                            ..pos.load(core::sync::atomic::Ordering::Relaxed) + address_size]
                            .as_array()
                            .expect("Not enough bytes"),
                    ) as u64,
                    Architecture::Bits64 => u64::from_be_bytes(
                        *bytes[pos.load(core::sync::atomic::Ordering::Relaxed)
                            ..pos.load(core::sync::atomic::Ordering::Relaxed) + address_size]
                            .as_array()
                            .expect("Not enough bytes"),
                    ),
                },
                Endianness::Little => match class {
                    Architecture::Bits32 => u32::from_le_bytes(
                        *bytes[pos.load(core::sync::atomic::Ordering::Relaxed)
                            ..pos.load(core::sync::atomic::Ordering::Relaxed) + address_size]
                            .as_array()
                            .expect("Not enough bytes"),
                    ) as u64,
                    Architecture::Bits64 => u64::from_le_bytes(
                        *bytes[pos.load(core::sync::atomic::Ordering::Relaxed)
                            ..pos.load(core::sync::atomic::Ordering::Relaxed) + address_size]
                            .as_array()
                            .expect("Not enough bytes"),
                    ),
                },
            };
            pos.fetch_add(address_size, core::sync::atomic::Ordering::Relaxed);
            ret
        };

        let ei_version = get_u8();
        let abi = get_u8();
        let abi_version = get_u8();
        pos.fetch_add(7, core::sync::atomic::Ordering::Relaxed);
        let file_type = get_u16().into();
        let isa = get_u16();
        let version = get_u32();
        let entry = get_u64();
        let phoff = get_u64();
        let shoff = get_u64();
        let flags = get_u32();
        let header_size = get_u16();
        let ph_entry_size = get_u16();
        let ph_entry_num = get_u16();
        let sh_entry_size = get_u16();
        let sh_entry_num = get_u16();
        let shstrndx = get_u16();

        let ret = Self {
            magic,
            class,
            endianness,
            ei_version,
            abi,
            abi_version,
            _padding: [0; 7],
            file_type,
            isa,
            version,
            entry,
            phoff,
            shoff,
            flags,
            header_size,
            ph_entry_size,
            ph_entry_num,
            sh_entry_size,
            sh_entry_num,
            shstrndx,
        };
        if pos.load(core::sync::atomic::Ordering::Relaxed) != ret.header_size as usize {
            panic!("pos ({}) should be {} ", pos.get_mut(), ret.header_size);
        }

        Ok(ret)
    }
}

#[repr(C)]
pub struct ProgramHeaderTable<'a> {
    class: Architecture,
    endianness: Endianness,
    entry_size: u16,
    entry_num: u16,
    bytes: &'a [u8],
}

impl<'a> ProgramHeaderTable<'a> {
    pub fn entries(&self) -> impl Iterator<Item = ProgramHeader> {
        let mut count: usize = 0;
        from_fn(move || {
            if count < self.entry_num as usize {
                let ret = Some(ProgramHeader::from_bytes(
                    self.class,
                    self.endianness,
                    &self.bytes
                        [count * self.entry_size as usize..(count + 1) * self.entry_size as usize],
                ));
                count += 1;
                ret
            } else {
                None
            }
        })
    }
}

#[derive(Debug)]
#[repr(C)]
pub struct ProgramHeader {
    pub segment_type: SegmentType,
    pub flags: u32,
    pub offset: u64,
    pub vaddr: u64,
    pub paddr: u64,
    pub file_size: u64,
    pub mem_size: u64,
    pub align: u64,
}

impl ProgramHeader {
    fn from_bytes(class: Architecture, endianness: Endianness, bytes: &[u8]) -> Self {
        let address_size = match class {
            Architecture::Bits32 => 4,
            Architecture::Bits64 => 8,
        };

        let pos = AtomicUsize::new(0);

        let get_u32 = || -> u32 {
            let ret = match endianness {
                Endianness::Big => u32::from_be_bytes(
                    *bytes[pos.load(core::sync::atomic::Ordering::Relaxed)
                        ..pos.load(core::sync::atomic::Ordering::Relaxed) + 4]
                        .as_array()
                        .expect("Not enough bytes"),
                ),
                Endianness::Little => u32::from_le_bytes(
                    *bytes[pos.load(core::sync::atomic::Ordering::Relaxed)
                        ..pos.load(core::sync::atomic::Ordering::Relaxed) + 4]
                        .as_array()
                        .expect("Not enough bytes"),
                ),
            };
            pos.fetch_add(4, core::sync::atomic::Ordering::Relaxed);
            ret
        };
        let get_u64 = || -> u64 {
            let ret = match endianness {
                Endianness::Big => match class {
                    Architecture::Bits32 => u32::from_be_bytes(
                        *bytes[pos.load(core::sync::atomic::Ordering::Relaxed)
                            ..pos.load(core::sync::atomic::Ordering::Relaxed) + address_size]
                            .as_array()
                            .expect("Not enough bytes"),
                    ) as u64,
                    Architecture::Bits64 => u64::from_be_bytes(
                        *bytes[pos.load(core::sync::atomic::Ordering::Relaxed)
                            ..pos.load(core::sync::atomic::Ordering::Relaxed) + address_size]
                            .as_array()
                            .expect("Not enough bytes"),
                    ),
                },
                Endianness::Little => match class {
                    Architecture::Bits32 => u32::from_le_bytes(
                        *bytes[pos.load(core::sync::atomic::Ordering::Relaxed)
                            ..pos.load(core::sync::atomic::Ordering::Relaxed) + address_size]
                            .as_array()
                            .expect("Not enough bytes"),
                    ) as u64,
                    Architecture::Bits64 => u64::from_le_bytes(
                        *bytes[pos.load(core::sync::atomic::Ordering::Relaxed)
                            ..pos.load(core::sync::atomic::Ordering::Relaxed) + address_size]
                            .as_array()
                            .expect("Not enough bytes"),
                    ),
                },
            };
            pos.fetch_add(address_size, core::sync::atomic::Ordering::Relaxed);
            ret
        };

        match class {
            Architecture::Bits32 => Self {
                segment_type: get_u32().into(),
                offset: get_u64(),
                vaddr: get_u64(),
                paddr: get_u64(),
                file_size: get_u64(),
                mem_size: get_u64(),
                flags: get_u32(),
                align: get_u64(),
            },
            Architecture::Bits64 => Self {
                segment_type: get_u32().into(),
                flags: get_u32(),
                offset: get_u64(),
                vaddr: get_u64(),
                paddr: get_u64(),
                file_size: get_u64(),
                mem_size: get_u64(),
                align: get_u64(),
            },
        }
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum Architecture {
    Bits32 = 1,
    Bits64 = 2,
}

impl From<u8> for Architecture {
    fn from(value: u8) -> Self {
        match value {
            1 => Architecture::Bits32,
            2 => Architecture::Bits64,
            _ => panic!("invalid architecture value"),
        }
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum Endianness {
    Little = 1,
    Big = 2,
}

impl From<u8> for Endianness {
    fn from(value: u8) -> Self {
        match value {
            1 => Endianness::Little,
            2 => Endianness::Big,
            _ => panic!("invalid endianness value"),
        }
    }
}

#[derive(Debug)]
#[repr(u16)]
pub enum FileType {
    None = 0,
    Relocatable = 1,
    Executable = 2,
    Dynamic = 3,
    Core = 4,
}

impl From<u16> for FileType {
    fn from(value: u16) -> Self {
        match value {
            0 => FileType::None,
            1 => FileType::Relocatable,
            2 => FileType::Executable,
            3 => FileType::Dynamic,
            4 => FileType::Core,
            _ => panic!("invalid file type"),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum SegmentType {
    Null = 0,
    Loadable = 1,
    DynamicLinking = 2,
    Interpreter = 3,
    Auxiliary = 4,
    Reserved = 5,
    ProgramHeader = 6,
    ThreadLocalStorage = 7,
    OtherReserved(u32),
}

impl From<u32> for SegmentType {
    fn from(value: u32) -> Self {
        match value {
            0 => SegmentType::Null,
            1 => SegmentType::Loadable,
            2 => SegmentType::DynamicLinking,
            3 => SegmentType::Interpreter,
            4 => SegmentType::Auxiliary,
            5 => SegmentType::Reserved,
            6 => SegmentType::ProgramHeader,
            7 => SegmentType::ThreadLocalStorage,
            val => SegmentType::OtherReserved(val),
        }
    }
}

pub struct SectionHeaderTable<'a> {
    class: Architecture,
    endianness: Endianness,
    entry_size: u16,
    entry_num: u16,
    bytes: &'a [u8],
}

impl<'a> SectionHeaderTable<'a> {
    pub fn entries(&self) -> impl Iterator<Item = SectionHeader> {
        let mut count: usize = 0;
        from_fn(move || {
            if count < self.entry_num as usize {
                let ret = Some(SectionHeader::from_bytes(
                    self.class,
                    self.endianness,
                    &self.bytes
                        [count * self.entry_size as usize..(count + 1) * self.entry_size as usize],
                ));
                count += 1;
                ret
            } else {
                None
            }
        })
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct SectionHeader {
    pub name_offset: u32,
    pub section_type: SectionType,
    pub flags: SectionFlags,
    pub addr: u64,
    pub offset: u64,
    pub size: u64,
    pub link: u32,
    pub info: u32,
    pub align: u64,
    pub entry_size: u64,
}

impl SectionHeader {
    pub fn from_bytes(class: Architecture, endianness: Endianness, bytes: &[u8]) -> Self {
        let address_size = match class {
            Architecture::Bits32 => 4,
            Architecture::Bits64 => 8,
        };

        let pos = AtomicUsize::new(0);

        let get_u32 = || -> u32 {
            let ret = match endianness {
                Endianness::Big => u32::from_be_bytes(
                    *bytes[pos.load(core::sync::atomic::Ordering::Relaxed)
                        ..pos.load(core::sync::atomic::Ordering::Relaxed) + 4]
                        .as_array()
                        .expect("Not enough bytes"),
                ),
                Endianness::Little => u32::from_le_bytes(
                    *bytes[pos.load(core::sync::atomic::Ordering::Relaxed)
                        ..pos.load(core::sync::atomic::Ordering::Relaxed) + 4]
                        .as_array()
                        .expect("Not enough bytes"),
                ),
            };
            pos.fetch_add(4, core::sync::atomic::Ordering::Relaxed);
            ret
        };
        let get_u64 = || -> u64 {
            let ret = match endianness {
                Endianness::Big => match class {
                    Architecture::Bits32 => u32::from_be_bytes(
                        *bytes[pos.load(core::sync::atomic::Ordering::Relaxed)
                            ..pos.load(core::sync::atomic::Ordering::Relaxed) + address_size]
                            .as_array()
                            .expect("Not enough bytes"),
                    ) as u64,
                    Architecture::Bits64 => u64::from_be_bytes(
                        *bytes[pos.load(core::sync::atomic::Ordering::Relaxed)
                            ..pos.load(core::sync::atomic::Ordering::Relaxed) + address_size]
                            .as_array()
                            .expect("Not enough bytes"),
                    ),
                },
                Endianness::Little => match class {
                    Architecture::Bits32 => u32::from_le_bytes(
                        *bytes[pos.load(core::sync::atomic::Ordering::Relaxed)
                            ..pos.load(core::sync::atomic::Ordering::Relaxed) + address_size]
                            .as_array()
                            .expect("Not enough bytes"),
                    ) as u64,
                    Architecture::Bits64 => u64::from_le_bytes(
                        *bytes[pos.load(core::sync::atomic::Ordering::Relaxed)
                            ..pos.load(core::sync::atomic::Ordering::Relaxed) + address_size]
                            .as_array()
                            .expect("Not enough bytes"),
                    ),
                },
            };
            pos.fetch_add(address_size, core::sync::atomic::Ordering::Relaxed);
            ret
        };

        Self {
            name_offset: get_u32(),
            section_type: get_u32().into(),
            flags: SectionFlags(get_u64()),
            addr: get_u64(),
            offset: get_u64(),
            size: get_u64(),
            link: get_u32(),
            info: get_u32(),
            align: get_u64(),
            entry_size: get_u64(),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[repr(u32)]
pub enum SectionType {
    Null = 0,
    ProgramData = 1,
    SymbolTable = 2,
    StringTable = 3,
    RelocationEntriesWithAddends = 4,
    SymbolHashTable = 5,
    DynamicLinking = 6,
    Notes = 7,
    BSS = 8,
    RelocationEntriesWithNoAddends = 9,
    Reserved = 10,
    DynamicLinkingSymbolTable = 11,
    Constructors = 14,
    Destructors = 15,
    PreConstructors = 16,
    SectionGroup = 17,
    ExtendedSectionIndices = 18,
    NumberDefinedTypes = 19,
    OtherReserved(u32),
}

impl From<u32> for SectionType {
    fn from(value: u32) -> Self {
        match value {
            0 => SectionType::Null,
            1 => SectionType::ProgramData,
            2 => SectionType::SymbolTable,
            3 => SectionType::StringTable,
            4 => SectionType::RelocationEntriesWithAddends,
            5 => SectionType::SymbolHashTable,
            6 => SectionType::DynamicLinking,
            7 => SectionType::Notes,
            8 => SectionType::BSS,
            9 => SectionType::RelocationEntriesWithNoAddends,
            10 => SectionType::Reserved,
            11 => SectionType::DynamicLinkingSymbolTable,
            14 => SectionType::Constructors,
            15 => SectionType::Destructors,
            16 => SectionType::PreConstructors,
            17 => SectionType::SectionGroup,
            18 => SectionType::ExtendedSectionIndices,
            19 => SectionType::NumberDefinedTypes,
            val => SectionType::OtherReserved(val),
        }
    }
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct SectionFlags(u64);

impl SectionFlags {
    pub fn writable(&self) -> bool {
        self.0 & 1 > 0
    }
    pub fn alloc(&self) -> bool {
        self.0 & 1 << 1 > 0
    }
    pub fn executable(&self) -> bool {
        self.0 & 1 << 2 > 0
    }
    pub fn merge(&self) -> bool {
        self.0 & 1 << 4 > 0
    }
    pub fn string(&self) -> bool {
        self.0 & 1 << 5 > 0
    }
    pub fn info_link(&self) -> bool {
        self.0 & 1 << 6 > 0
    }
    pub fn link_order(&self) -> bool {
        self.0 & 1 << 7 > 0
    }
    pub fn group(&self) -> bool {
        self.0 & 1 << 9 > 0
    }
    pub fn tls(&self) -> bool {
        self.0 & 1 << 10 > 0
    }
}

impl Debug for SectionFlags {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("SectionFlags")
            .field("W", &self.writable())
            .field("A", &self.alloc())
            .field("X", &self.executable())
            .field("M", &self.merge())
            .field("S", &self.string())
            .field("I", &self.info_link())
            .field("L", &self.link_order())
            .field("G", &self.group())
            .field("T", &self.tls())
            .finish()
    }
}

pub struct StringTable<'a> {
    pub header: SectionHeader,
    pub bytes: &'a [u8],
}

impl<'a> StringTable<'a> {
    pub fn get_name(&self, offset: usize) -> Option<&str> {
        let str_len = {
            let mut len = 0;
            while self.bytes[offset + len] != 0 {
                len += 1;
            }
            len
        };
        let name = str::from_utf8(&self.bytes[offset..offset + str_len]);
        if name.is_ok() {
            Some(name.ok()?)
        } else {
            None
        }
    }
}
