use core::fmt::Debug;

#[repr(C, packed(4))]
pub struct TaskStateSegment {
    reserved_0: u32,
    pub rsp: [u64; 3],
    reserved_1: u64,
    pub ist: [u64; 7],
    reserved_2: u64,
    pub iopb: u16,
}

impl TaskStateSegment {
    pub fn new() -> Self {
        Self {
            reserved_0: 0,
            rsp: [0; 3],
            reserved_1: 0,
            ist: [0; 7],
            reserved_2: 0,
            iopb: size_of::<TaskStateSegment>() as u16,
        }
    }
}

impl Debug for TaskStateSegment {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let rsp = self.rsp;
        let ist = self.ist;
        f.debug_struct("TaskStateSegment")
            .field("rsp", &rsp)
            .field("ist", &ist)
            .field("iopb", &self.iopb)
            .finish()
    }
}
