mod entry;
use core::{
    marker::PhantomData,
    ops::{Index, IndexMut, Sub},
};

use entry::Entry;

pub const PAGE_SIZE: usize = 4096;
pub const ENTRY_COUNT: usize = 512;

pub const P4: *mut Table<Level4> = 0xffffffff_fffff000 as *mut _;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct VirtualAddress(pub usize);

impl VirtualAddress {
    #[inline]
    pub const fn new_truncate(addr: usize) -> VirtualAddress {
        // By doing the right shift as a signed operation (on a i64), it will
        // sign extend the value, repeating the leftmost bit.
        VirtualAddress(((addr << 16) as i64 >> 16) as usize)
    }

    #[inline]
    pub fn new(addr: usize) -> VirtualAddress {
        let v = Self::new_truncate(addr);

        if v.0 == addr {
            v
        } else {
            panic!("Invalid virtual address")
        }
    }

    #[inline]
    pub const fn page_table_index(self, level: usize) -> usize {
        assert!(level < 5 && level > 0);
        (self.0 >> 12 >> ((level as u8 - 1) * 9)) % ENTRY_COUNT
    }

    #[cfg(target_pointer_width = "64")]
    #[inline]
    pub const fn as_ptr<T>(self) -> *const T {
        self.0 as *const T
    }

    #[cfg(target_pointer_width = "64")]
    #[inline]
    pub const fn as_mut_ptr<T>(self) -> *mut T {
        self.as_ptr::<T>() as *mut T
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct PhysicalAddress(pub usize);

impl Sub for PhysicalAddress {
    type Output = usize;
    fn sub(self, rhs: Self) -> Self::Output {
        self.0 - rhs.0
    }
}

#[derive(Clone, Copy)]
pub struct Page {
    pub start_address: VirtualAddress,
}

impl Page {
    fn containing_address(address: VirtualAddress) -> Page {
        Self {
            start_address: VirtualAddress((address.0 / PAGE_SIZE) * PAGE_SIZE),
        }
    }

    #[inline]
    pub const fn page_table_index(self, level: usize) -> usize {
        self.start_address.page_table_index(level)
    }

    #[inline]
    pub fn from_page_table_indices(
        p4_index: usize,
        p3_index: usize,
        p2_index: usize,
        p1_index: usize,
    ) -> Self {
        for index in [p1_index, p2_index, p3_index, p4_index] {
            assert!(index < ENTRY_COUNT, "Invalid index {index}")
        }
        let mut addr = 0;
        addr |= p4_index << 39;
        addr |= p3_index << 30;
        addr |= p2_index << 21;
        addr |= p1_index << 12;
        Page::containing_address(VirtualAddress::new_truncate(addr))
    }
}

pub trait TableLevel {}

pub enum Level4 {}
pub enum Level3 {}
pub enum Level2 {}
pub enum Level1 {}

impl TableLevel for Level4 {}
impl TableLevel for Level3 {}
impl TableLevel for Level2 {}
impl TableLevel for Level1 {}

pub trait HierarchicalLevel: TableLevel {
    type NextLevel: TableLevel;
}

impl HierarchicalLevel for Level4 {
    type NextLevel = Level3;
}

impl HierarchicalLevel for Level3 {
    type NextLevel = Level2;
}

impl HierarchicalLevel for Level2 {
    type NextLevel = Level1;
}

pub trait TopLevel: TableLevel {}
impl TopLevel for Level4 {}

#[derive(Clone, Copy)]
pub struct Table<L: TableLevel> {
    entries: [Entry; ENTRY_COUNT],
    phantom: PhantomData<L>,
}

impl<L> Table<L>
where
    L: TableLevel,
{
    pub fn zero(&mut self) {
        for entry in self.entries.iter_mut() {
            entry.set_unused();
        }
    }
}

impl<L> Table<L>
where
    L: HierarchicalLevel,
{
    fn next_table_address(&self, index: usize) -> Option<usize> {
        let entry_flags = self[index].get_flags();
        if entry_flags.present() && !entry_flags.huge() {
            let table_address = self as *const _ as usize;
            Some((table_address << 9) | (index << 12))
        } else {
            None
        }
    }

    pub fn next_table(&self, index: usize) -> Option<&Table<L>> {
        self.next_table_address(index)
            .map(|address| unsafe { &*(address as *const _) })
    }

    pub fn next_table_mut(&mut self, index: usize) -> Option<&mut Table<L>> {
        self.next_table_address(index)
            .map(|address| unsafe { &mut *(address as *mut _) })
    }
}

impl<L> Index<usize> for Table<L>
where
    L: TableLevel,
{
    type Output = Entry;

    fn index(&self, index: usize) -> &Entry {
        &self.entries[index]
    }
}

impl<L> IndexMut<usize> for Table<L>
where
    L: TableLevel,
{
    fn index_mut(&mut self, index: usize) -> &mut Entry {
        &mut self.entries[index]
    }
}

pub trait Mapper {
    fn to_virtual(&self, addr: PhysicalAddress) -> VirtualAddress;

    fn to_physical(&self, addr: VirtualAddress) -> PhysicalAddress;
}

pub struct RecursiveMapper<'a> {
    p4: &'a mut Table<Level4>,
}

impl<'a> RecursiveMapper<'a> {
    pub fn get_active() -> Self {
        Self {
            p4: unsafe { P4.as_mut().unwrap() },
        }
    }

    pub fn get_p4(&self) -> &Table<Level4> {
        self.p4
    }

    pub fn get_p4_mut(&mut self) -> &Table<Level4> {
        self.p4
    }
}

impl Mapper for RecursiveMapper<'_> {
    fn to_physical(&self, _addr: VirtualAddress) -> PhysicalAddress {
        unimplemented!()
    }
    fn to_virtual(&self, _addr: PhysicalAddress) -> VirtualAddress {
        unimplemented!()
    }
}
