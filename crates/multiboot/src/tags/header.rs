use crate::tags::TagType;

#[derive(Debug)]
#[repr(C)]
pub struct TagHeader {
    pub tag_type: TagType,
    pub(crate) size: u32,
}

impl TagHeader {
    pub fn from_bytes(bytes: &[u8; 8]) -> &Self {
        unsafe { &*(bytes as *const _ as *const Self) }
    }
}