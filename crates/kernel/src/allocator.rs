use core::{alloc::GlobalAlloc, fmt::Write, ptr};

use spin::{Mutex, MutexGuard};

use crate::{RING_BUFFER, log};

#[global_allocator]
pub static ALLOCATOR: Locked<BumpAllocator> = Locked::new(BumpAllocator::new());

pub fn init(heap_start: usize, heap_end: usize) {
    unsafe {
        ALLOCATOR.lock().init(heap_start, heap_end);
    }
}

pub struct Locked<T> {
    inner: Mutex<T>,
}

impl<T> Locked<T> {
    pub const fn new(t: T) -> Self {
        Self {
            inner: Mutex::new(t),
        }
    }
    pub fn lock(&self) -> MutexGuard<'_, T> {
        self.inner.lock()
    }
}

pub struct BumpAllocator {
    heap_start: usize,
    heap_end: usize,
    next: usize,
    allocations: usize,
}

impl BumpAllocator {
    const fn new() -> Self {
        Self {
            heap_start: 0,
            heap_end: 0,
            next: 0,
            allocations: 0,
        }
    }
    unsafe fn init(&mut self, start: usize, end: usize) {
        self.heap_start = start;
        self.heap_end = end;
        self.next = start;
        self.allocations = 0;
    }
}

impl Allocator for BumpAllocator {
    unsafe fn alloc(&mut self, layout: core::alloc::Layout) -> *mut u8 {
        let alloc_start = align_up(self.next, layout.align());

        let alloc_end = match alloc_start.checked_add(layout.size()) {
            Some(end) => end,
            None => return ptr::null_mut(),
        };

        if alloc_end > self.heap_end {
            ptr::null_mut()
        } else {
            self.next = alloc_end;
            self.allocations += 1;
            alloc_start as *mut u8
        }
    }

    unsafe fn dealloc(&mut self, _ptr: *mut u8, _layout: core::alloc::Layout) {
        self.allocations -= 1;
        if self.allocations == 0 {
            self.next = self.heap_start;
        }
    }
}

fn align_up(addr: usize, align: usize) -> usize {
    let remainder = addr & align;
    if remainder == 0 {
        addr
    } else {
        addr - remainder + align
    }
}

unsafe impl<A> GlobalAlloc for Locked<A>
where
    A: Allocator,
{
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        unsafe { self.lock().alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: core::alloc::Layout) {
        unsafe { self.lock().dealloc(ptr, layout) };
    }
}

pub trait Allocator {
    unsafe fn alloc(&mut self, layout: core::alloc::Layout) -> *mut u8;

    unsafe fn dealloc(&mut self, ptr: *mut u8, layout: core::alloc::Layout);
}
