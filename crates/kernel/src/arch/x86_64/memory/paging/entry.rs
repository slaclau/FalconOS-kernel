use core::fmt::Debug;

use crate::utils::bits::get_bit;

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Entry(u64);

impl Entry {
    pub fn new() -> Self {
        Self(0)
    }

    pub fn unused(&self) -> bool {
        self.0 == 0
    }

    pub fn set_unused(&mut self) {
        self.0 = 0;
    }

    pub fn get_flags(self) -> Flags {
        Flags::from_entry(self)
    }

    pub fn pointed_address(&self) -> Option<usize> {
        if self.get_flags().present() {
            Some(self.0 as usize & 0x000fffff_fffff000)
        } else {
            None
        }
    }

    pub fn set(&mut self, address: usize, flags: Flags) {
        assert!(address & !0x000fffff_fffff000 == 0);
        self.0 = address as u64 | flags.0;
    }
}

pub struct Flags(u64);

impl Flags {
    pub fn from_entry(entry: Entry) -> Self {
        Self(entry.0 & (0x1FF | (1 << 63)))
    }

    pub fn present(&self) -> bool {
        get_bit(self.0, 0) > 0
    }

    pub fn writable(&self) -> bool {
        get_bit(self.0, 1) > 0
    }

    pub fn user_accessible(&self) -> bool {
        get_bit(self.0, 2) > 0
    }

    pub fn write_through_caching(&self) -> bool {
        get_bit(self.0, 3) > 0
    }

    pub fn disable_caching(&self) -> bool {
        get_bit(self.0, 4) > 0
    }

    pub fn accessed(&self) -> bool {
        get_bit(self.0, 5) > 0
    }

    pub fn dirty(&self) -> bool {
        get_bit(self.0, 6) > 0
    }

    pub fn huge(&self) -> bool {
        get_bit(self.0, 7) > 0
    }

    pub fn global(&self) -> bool {
        get_bit(self.0, 8) > 0
    }

    pub fn no_execute(&self) -> bool {
        get_bit(self.0, 63) > 0
    }
}

impl Debug for Flags {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("paging::entry::Flags")
            .field("P", &self.present())
            .field("RW", &self.writable())
            .field("U", &self.user_accessible())
            .field("PWT", &self.write_through_caching())
            .field("PCD", &self.disable_caching())
            .field("A", &self.accessed())
            .field("D", &self.dirty())
            .field("PS", &self.huge())
            .field("G", &self.global())
            .field("XD", &self.no_execute())
            .finish()
    }
}
