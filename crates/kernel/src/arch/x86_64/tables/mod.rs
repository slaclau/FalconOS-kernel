pub mod gdt;
pub mod idt;

#[repr(u16)]
#[derive(Debug)]
pub enum PrivilegeLevel {
    Ring0 = 0,
    Ring1 = 1,
    Ring2 = 2,
    Ring3 = 3,
}

impl PrivilegeLevel {
    pub fn from_u16(value: u16) -> Self {
        match value {
            0 => PrivilegeLevel::Ring0,
            1 => PrivilegeLevel::Ring1,
            2 => PrivilegeLevel::Ring2,
            3 => PrivilegeLevel::Ring3,
            _ => panic!("Should be unreachable"),
        }
    }
}

#[repr(C, packed(2))]
#[derive(Debug)]
pub struct TablePointer {
    pub size: u16,
    pub offset: u64,
}
