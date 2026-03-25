use crate::arch::x86_64::port::{self, Port};

/// Command sent to begin PIC initialization.
const CMD_INIT: u8 = 0x11;

/// Command sent to acknowledge an interrupt.
const CMD_END_OF_INTERRUPT: u8 = 0x20;

// The mode in which we want to run our PICs.
const MODE_8086: u8 = 0x01;

pub struct ChainedPics {
    master: Pic,
    slave: Pic,
}

impl ChainedPics {
    pub unsafe fn new(master_offset: u8, slave_offset: u8) -> Self {
        Self {
            master: Pic {
                offset: master_offset,
                command: Port::new_readwrite(0x20),
                data: Port::new_readwrite(0x21),
            },
            slave: Pic {
                offset: slave_offset,
                command: Port::new_readwrite(0xA0),
                data: Port::new_readwrite(0xA1),
            },
        }
    }

    /// Initialize both our PICs.  We initialize them together, at the same
    /// time, because it's traditional to do so, and because I/O operations
    /// might not be instantaneous on older processors.
    pub unsafe fn initialize(&mut self) {
        unsafe {
            // We need to add a delay between writes to our PICs, especially on
            // older motherboards.  But we don't necessarily have any kind of
            // timers yet, because most of them require interrupts.  Various
            // older versions of Linux and other PC operating systems have
            // worked around this by writing garbage data to port 0x80, which
            // allegedly takes long enough to make everything work on most
            // hardware.  Here, `wait` is a closure.
            let mut wait_port: Port<u8> = port::Port::new_readwrite(0x80);
            let mut wait = || wait_port.write(0);

            // Save our original interrupt masks, because I'm too lazy to
            // figure out reasonable values.  We'll restore these when we're
            // done.
            let saved_masks = self.read_masks();

            // Tell each PIC that we're going to send it a three-byte
            // initialization sequence on its data port.
            self.master.command.write(CMD_INIT);
            wait();
            self.slave.command.write(CMD_INIT);
            wait();

            // Byte 1: Set up our base offsets.
            self.master.data.write(self.master.offset);
            wait();
            self.slave.data.write(self.slave.offset);
            wait();

            // Byte 2: Configure chaining between PIC1 and PIC2.
            self.master.data.write(4);
            wait();
            self.slave.data.write(2);
            wait();

            // Byte 3: Set our mode.
            self.master.data.write(MODE_8086);
            wait();
            self.slave.data.write(MODE_8086);
            wait();

            // Restore our saved masks.
            self.write_masks(saved_masks[0], saved_masks[1])
        }
    }

    /// Reads the interrupt masks of both PICs.
    pub unsafe fn read_masks(&mut self) -> [u8; 2] {
        unsafe { [self.master.read_mask(), self.slave.read_mask()] }
    }

    /// Writes the interrupt masks of both PICs.
    pub unsafe fn write_masks(&mut self, mask1: u8, mask2: u8) {
        unsafe {
            self.master.write_mask(mask1);
            self.slave.write_mask(mask2);
        }
    }

    pub unsafe fn is_masked(&mut self, port: u8) -> bool {
        if port < 8 {
            unsafe { self.master.is_masked(port) }
        } else {
            unsafe { self.slave.is_masked(port - 8) }
        }
    }

    pub unsafe fn set_masked(&mut self, port: u8, masked: bool) {
        if port < 8 {
            unsafe { self.master.set_masked(port, masked) }
        } else {
            unsafe { self.slave.set_masked(port - 8, masked) }
        }
    }

    /// Do we handle this interrupt?
    pub fn handles_interrupt(&self, interrupt_id: u8) -> bool {
        self.master.handles_interrupt(interrupt_id) || self.slave.handles_interrupt(interrupt_id)
    }

    /// Figure out which (if any) PICs in our chain need to know about this
    /// interrupt.  This is tricky, because all interrupts from `pics[1]`
    /// get chained through `pics[0]`.
    pub unsafe fn notify_end_of_interrupt(&mut self, interrupt_id: u8) {
        if self.handles_interrupt(interrupt_id) {
            if self.slave.handles_interrupt(interrupt_id) {
                unsafe { self.slave.end_of_interrupt() };
            }
            unsafe {
                self.master.end_of_interrupt();
            }
        }
    }
}

struct Pic {
    offset: u8,
    command: Port<u8>,
    data: Port<u8>,
}

impl Pic {
    fn handles_interrupt(&self, id: u8) -> bool {
        self.offset <= id && id < self.offset + 8
    }

    /// Notify us that an interrupt has been handled and that we're ready
    /// for more.
    unsafe fn end_of_interrupt(&mut self) {
        unsafe { self.command.write(CMD_END_OF_INTERRUPT) };
    }

    /// Reads the interrupt mask of this PIC.
    unsafe fn read_mask(&mut self) -> u8 {
        unsafe { self.data.read() }
    }

    /// Writes the interrupt mask of this PIC.
    unsafe fn write_mask(&mut self, mask: u8) {
        unsafe { self.data.write(mask) }
    }

    unsafe fn is_masked(&mut self, port: u8) -> bool {
        unsafe { self.read_mask() & (1 << port) > 0 }
    }

    unsafe fn set_masked(&mut self, port: u8, masked: bool) {
        let old_mask = unsafe { self.read_mask() & (1 << port) };
        let new_mask;
        if masked {
            new_mask = old_mask | (1 << port);
        } else {
            new_mask = old_mask & !(1 << port);
        }
        unsafe {
            self.write_mask(new_mask);
        }
    }
}
