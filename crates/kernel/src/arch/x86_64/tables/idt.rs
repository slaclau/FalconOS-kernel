use core::{
    arch::asm,
    fmt::{self, Debug},
};

use crate::arch::x86_64::tables::{PrivilegeLevel, TablePointer};

const IDT_LENGTH: usize = 256;

const DOUBLE_FAULT_INDEX: usize = 0x8;
const GP_FAULT_INDEX: usize = 0xd;
const PAGE_FAULT_INDEX: usize = 0xe;


#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Table {
    entries: [Entry; IDT_LENGTH],
}

impl Table {
    pub fn new() -> Self {
        Self {
            entries: [Entry::empty(); IDT_LENGTH],
        }
    }
    pub fn double_fault(&mut self) -> &mut Entry {
        &mut self.entries[DOUBLE_FAULT_INDEX]
    }
    pub fn gp_fault(&mut self) -> &mut Entry {
        &mut self.entries[GP_FAULT_INDEX]
    }
    pub fn page_pault(&mut self) -> &mut Entry {
        &mut self.entries[PAGE_FAULT_INDEX]
    }

    pub fn interrupts(&mut self) -> &mut [Entry] {
        &mut self.entries[32..IDT_LENGTH]
    }
    pub fn pointer(&self) -> TablePointer {
        TablePointer {
            offset: self as *const _ as u64,
            size: size_of::<Self>() as u16 - 1,
        }
    }

    pub unsafe fn load(&'static self) {
        let idt = &self.pointer();
        unsafe {
            asm!("lidt [{}]", in(reg) idt, options(readonly, nostack, preserves_flags));
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Entry {
    pointer_low: u16,
    gdt_selector: u16,
    pub options: EntryOptions,
    pointer_middle: u16,
    pointer_high: u32,
    reserved: u32,
}
impl Entry {
    fn new(pointer: u64, gdt_selector: u16, options: EntryOptions) -> Self {
        Self {
            pointer_low: pointer as u16,
            gdt_selector,
            options,
            pointer_middle: (pointer >> 16) as u16,
            pointer_high: (pointer >> 32) as u32,
            reserved: 0,
        }
    }
    fn empty() -> Self {
        let cs: u16;
        unsafe {
            asm!("mov {0:x}, cs", out(reg) cs, options(nomem, nostack, preserves_flags));
        };
        Self::new(0, cs, EntryOptions::minimal())
    }

    pub unsafe fn set_handler_addr(&mut self, handler_addr: u64) {
        self.pointer_low = handler_addr as u16;
        self.pointer_middle = (handler_addr >> 16) as u16;
        self.pointer_high = (handler_addr >> 32) as u32;
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct EntryOptions(u16);

impl EntryOptions {
    fn minimal() -> Self {
        Self::new(0, GateType::Interrupt, PrivilegeLevel::Ring0, false)
    }
    fn new(stack_index: u8, gate_type: GateType, dpl: PrivilegeLevel, present: bool) -> Self {
        let bits = stack_index as u16
            | (gate_type as u16) << 8
            | (dpl as u16) << 13
            | (present as u16) << 15;
        Self(bits)
    }

    pub fn get_stack_index(self) -> u8 {
        ((self.0 << 13) >> 13) as u8
    }

    pub fn set_stack_index(&mut self, stack_index: u8) -> &mut Self {
        assert!(stack_index <= 7);
        self.0 = (self.0 >> 3) << 3 | stack_index as u16;
        self
    }

    pub fn get_dpl(self) -> PrivilegeLevel {
        let dpl = (self.0 & 0b0110000000000000) >> 13;

        PrivilegeLevel::from_u16(dpl)
    }

    pub fn set_dpl(&mut self, dpl: PrivilegeLevel) -> &mut Self {
        self.0 = (self.0 & 0b1001111111111111) | ((dpl as u16) << 13);
        self
    }

    pub fn get_gate_type(self) -> GateType {
        let gate_type = (self.0 >> 8) & 0xF;

        match gate_type {
            0xE => GateType::Interrupt,
            0xF => GateType::Trap,
            _ => panic!("Should be unreachable"),
        }
    }

    pub fn set_gate_type(&mut self, gate_type: GateType) -> &mut Self {
        self.0 = (self.0 & 0xF0F0) | ((gate_type as u16) << 8);

        self
    }

    pub fn get_present(self) -> bool {
        self.0 >> 15 != 0
    }

    pub fn set_present(&mut self, present: bool) -> &mut Self {
        self.0 = (self.0 << 1) >> 1 | ((present as u16) << 15);
        self
    }
}

impl fmt::Debug for EntryOptions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EntryOptions")
            .field("bits", &format_args!("{:#018b}", self.0))
            .field("present", &self.get_present())
            .field("gate_type", &self.get_gate_type())
            .field("dpl", &self.get_dpl())
            .field("stack_index", &self.get_stack_index())
            .finish()
    }
}

#[repr(u16)]
#[derive(Debug)]
pub enum GateType {
    Interrupt = 0xE,
    Trap = 0xF,
}

#[repr(C)]
#[derive(Debug)]
pub struct StackFrame {
    ip: usize,
    cs: usize,
    flags: usize,
    sp: usize,
    ss: usize,
}

#[repr(C)]
pub struct PageFaultErrorCode(pub u64);

impl PageFaultErrorCode {
    fn is_present(&self) -> bool {
        self.0 & 0b1 != 0
    }

    fn is_write(&self) -> bool {
        self.0 & 0b10 != 0
    }

    fn is_user(&self) -> bool {
        self.0 & 0b100 != 0
    }

    fn is_instruction_fetch(&self) -> bool {
        self.0 & 0b1000 != 0
    }

    fn is_protection_key(&self) -> bool {
        self.0 & 0b10000 != 0
    }

    fn is_shadow_stack(&self) -> bool {
        self.0 & 0b100000 != 0
    }

    fn is_sgx(&self) -> bool {
        self.0 & 0x8000 != 0
    }
}

impl Debug for PageFaultErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PageFaultErrorCode")
            .field("P", &self.is_present())
            .field("W", &self.is_write())
            .field("U", &self.is_user())
            .field("I", &self.is_instruction_fetch())
            .field("PK", &self.is_protection_key())
            .field("SS", &self.is_shadow_stack())
            .field("SGX", &self.is_sgx())
            .finish()
    }
}
