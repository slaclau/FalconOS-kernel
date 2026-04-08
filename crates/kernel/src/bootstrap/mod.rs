use core::fmt::{Debug, Write};

use crate::{PhysicalAddress, process::Process};
use elf::Elf;
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

    let initial_file = initial_file.unwrap();

    let bytes = initial_file.bytes;
    log!(
        RING_BUFFER,
        "found initial file {}",
        initial_file.header_record.path()
    );

    let _elf = Elf(bytes);

    let dummy_task = Process::new(dummy_task);
    log!(RING_BUFFER, "{dummy_task:#x?}");
    dummy_task.register();

    let bs_task = Process::new(bs_task);
    log!(RING_BUFFER, "{bs_task:#x?}");
    let bs_task_id = bs_task.register();

    let bs_task2 = Process::new(bs_task2);
    log!(RING_BUFFER, "{bs_task2:#x?}");
    bs_task2.register();

    syscall::switch(bs_task_id);
}

extern "C" fn dummy_task() {}

extern "C" fn bs_task() {
    let pid = syscall::get_pid();
    log!(RING_BUFFER, "bs 1 started with pid {pid}");
    let mut i = 0;
    loop {
        i += 1;
        if i % 10000000 == 0 {
            log!(RING_BUFFER, "bs 1 yielding");
            let prev = syscall::switch(2);
            log!(RING_BUFFER, "bs 1 got control back from {prev}");
        }
    }
}

extern "C" fn bs_task2() {
    let pid = syscall::get_pid();
    log!(RING_BUFFER, "bs 2 started with pid {pid}");
    let mut i = 0;
    loop {
        i += 1;
        if i % 10000000 == 0 {
            log!(RING_BUFFER, "bs 2 yielding");
            let prev = syscall::switch(1);
            log!(RING_BUFFER, "bs 2 got control back from {prev}");
        }
    }
}
