use core::fmt::{Debug, Write};

use crate::{
    PhysicalAddress,
    capability::{self, Rights, create_endpoint, derive_cap, move_cap, send},
    ipc::Message,
    process::{KERNEL_TASK_ID, Process},
};
use elf::{Elf, SegmentType};
use tar::Archive;

use crate::{BOOTSTRAP_INFO, RING_BUFFER, log};
pub struct Info<'a> {
    pub tarfs: Option<&'a [u8]>,
    pub command_line: [u8; 128],
    pub kernel_memory_region: Option<[PhysicalAddress; 2]>,
    pub available_memory_regions: [Option<[PhysicalAddress; 2]>; 16],
}

impl<'a> Debug for Info<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Info")
            .field_with("command_line", |f| {
                f.write_fmt(format_args!(
                    "{}",
                    str::from_utf8(&self.command_line).unwrap()
                ))
            })
            .finish()
    }
}

pub fn run() {
    let info = BOOTSTRAP_INFO.lock();
    log!(
        RING_BUFFER,
        "running userspace bootstrap with args {:?}",
        &info
    );

    let archive = Archive(info.tarfs.unwrap());
    let command_line = str::from_utf8(info.command_line.as_slice())
        .unwrap()
        .trim_matches(char::from(0));

    let initial_file = archive.files().find(|f| {
        &f.header_record.path()[1..] == command_line && f.header_record.path().starts_with(".")
    });

    if initial_file.is_none() {
        panic!("No initial bootstrap file matching {} found", command_line);
    }

    for area in &info.available_memory_regions {
        if area.is_some() {
            let area = area.unwrap();
            log!(
                RING_BUFFER,
                "Available memory: {:#x} - {:#x}",
                area[0].0,
                area[1].0
            );
        }
    }
    log!(
        RING_BUFFER,
        "Memory in use by OS currently: {:#x} - {:#x}",
        info.kernel_memory_region.unwrap()[0].0,
        info.kernel_memory_region.unwrap()[1].0
    );

    capability::init(info.kernel_memory_region, info.available_memory_regions);

    let initial_file = initial_file.unwrap();

    let bytes = initial_file.bytes;
    log!(
        RING_BUFFER,
        "found initial file {}",
        initial_file.header_record.path()
    );

    let elf = Elf(bytes);

    for s in elf.program_header().entries() {
        if s.segment_type == SegmentType::Loadable && s.vaddr > 0 {
            log!(RING_BUFFER, "{s:?}");
            unsafe {
                core::ptr::copy(
                    (&bytes[0] as *const u8).add(s.offset as usize),
                    s.vaddr as *mut u8,
                    s.file_size as usize,
                );
            }
        }
    }

    let bs = syscall::spawn(
        unsafe {
            core::mem::transmute::<u64, extern "C" fn(arg: usize) -> usize>(elf.header().entry)
        },
        0,
    );

    let ep_id = create_endpoint(KERNEL_TASK_ID).unwrap();
    log!(
        RING_BUFFER,
        "created endpoint {ep_id} for proc {KERNEL_TASK_ID}"
    );

    let recv_ep_id = derive_cap(KERNEL_TASK_ID, ep_id, Rights::READ).unwrap();
    log!(
        RING_BUFFER,
        "created endpoint {recv_ep_id} for proc {KERNEL_TASK_ID}"
    );

    move_cap(KERNEL_TASK_ID, recv_ep_id, bs).unwrap();
    Process::get_mut(bs).dump_caps();
    Process::get_mut(KERNEL_TASK_ID).dump_caps();

    let bytes = "sendsend".as_bytes();
    let data = usize::from_be_bytes(*bytes.as_array().unwrap());

    send(
        KERNEL_TASK_ID,
        ep_id,
        Message {
            data,
        },
    )
    .expect("could not send");

    let done = syscall::switch(bs);

    log!(RING_BUFFER, "back to kernel control from {done}");

    let exit_code = syscall::wait(bs);
    log!(RING_BUFFER, "bs exited with {exit_code}");
}
