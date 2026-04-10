#![no_std]
#![feature(abi_x86_interrupt)]
#![feature(debug_closure_helpers)]
#![feature(ptr_metadata)]
#![allow(static_mut_refs)]

use core::{fmt::Write, ptr::from_raw_parts, slice};

extern crate alloc;

use elf::SectionHeader;
use multiboot::{ElfSectionsTag, MemoryMapTagEntryType, ModuleTag};
use spin::Mutex;

use crate::utils::ring_buffer::{RING_BUFFER_LENGTH, RingBuffer};

mod allocator;
mod arch;
mod bootstrap;
mod process;
mod syscall;
mod utils;

pub use arch::*;

pub static RING_BUFFER: Mutex<RingBuffer<RING_BUFFER_LENGTH>> =
    Mutex::new(RingBuffer::<RING_BUFFER_LENGTH>::new());

pub static BOOTSTRAP_INFO: Mutex<bootstrap::Info> = Mutex::new(bootstrap::Info {
    tarfs: None,
    command_line: [0; 128],
    available_memory_regions: [const { None }; 16],
    kernel_memory_region: None,
});

pub const HEAP_SIZE: usize = 4096 * 32;
pub static HEAP: [u8; HEAP_SIZE] = [0; HEAP_SIZE];

#[cfg(debug_assertions)]
mod debug;

pub fn kernel_main() -> ! {
    log!(RING_BUFFER, "kernel_main called");

    let heap_start = &HEAP as *const _ as usize;
    allocator::init(heap_start, heap_start + HEAP_SIZE);
    log!(RING_BUFFER, "global allocator init called");

    process::init_multiprocessing();

    bootstrap::run();

    loop {
        hal::halt();
    }
}

pub fn kernel_shared_init(mb_ptr: u32, mb_magic: u32) {
    log!(RING_BUFFER, "kernel_shared_init called");

    match mb_magic {
        multiboot::MULTIBOOT2_MAGIC => {
            log!(RING_BUFFER, "multiboot 2 detected");

            prepare_bootstrap_info_mb2(mb_ptr);
        }
        _ => {
            unimplemented!()
        }
    }
}

fn prepare_bootstrap_info_mb2(mb_ptr: u32) {
    let information = unsafe { multiboot::BootInformation::load(mb_ptr) };

    log!(
        RING_BUFFER,
        "Booted with command line: {}",
        information.command_line().string()
    );
    log!(
        RING_BUFFER,
        "Booted by: {}",
        information.boot_loader_name().string()
    );

    let module = information.get_tag::<ModuleTag>();
    let bytes_ptr: *const [u8] = from_raw_parts(
        module.start as *const u8,
        (module.end - module.start) as usize,
    );
    let bytes = unsafe { &*bytes_ptr };

    let memory_map = &information.memory_map().entries;
    let available_memory_regions = memory_map
        .iter()
        .filter(|entry| entry.memory_area_type == MemoryMapTagEntryType::Available)
        .map(|entry| {
            [
                PhysicalAddress(entry.base_addr as usize),
                PhysicalAddress((entry.base_addr + entry.length) as usize),
            ]
        });

    let elf_sections_tag = information.get_tag::<ElfSectionsTag>();
    let string_table_header = elf_sections_tag
        .entries()
        .nth(elf_sections_tag.shndx as usize)
        .expect("String table not where expected");
    let string_table = elf::StringTable {
        header: string_table_header,
        bytes: unsafe {
            slice::from_raw_parts(
                string_table_header.addr as *const _,
                string_table_header.size as usize,
            )
        },
    };

    let needed_sections_filter = |section: &SectionHeader| {
        let name = string_table.get_name(section.name_offset as usize);
        if name.is_some() && name.unwrap() == ".loader" {
            false
        } else {
            section.flags.alloc()
        }
    };
    let kernel_start = elf_sections_tag
        .entries()
        .filter(needed_sections_filter)
        .map(|section| section.addr)
        .min()
        .unwrap();
    let kernel_end = elf_sections_tag
        .entries()
        .filter(needed_sections_filter)
        .map(|section| section.addr + section.size)
        .max()
        .unwrap();

    let mut bootstrap_info = BOOTSTRAP_INFO.lock();

    let mut command_line = [0; 128];
    for (i, byte) in information
        .command_line()
        .string()
        .as_bytes()
        .iter()
        .enumerate()
    {
        command_line[i] = *byte;
    }
    bootstrap_info.tarfs = Some(bytes);
    bootstrap_info.command_line = command_line;
    bootstrap_info.kernel_memory_region = Some([
        PhysicalAddress(kernel_start as usize),
        PhysicalAddress(kernel_end as usize),
    ]);
    for (i, memory_region) in available_memory_regions.enumerate() {
        bootstrap_info.available_memory_regions[i] = Some(memory_region);
    }
}
