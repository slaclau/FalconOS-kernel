#![no_std]

use core::iter::from_fn;

pub struct Archive<'a>(pub &'a [u8]);

impl<'a> Archive<'a> {
    pub fn files(&self) -> impl Iterator<Item = File<'a>> {
        const RECORD_SIZE: usize = 512;
        let (_, remainder) = self.0.as_chunks::<RECORD_SIZE>();
        assert!(remainder.len() == 0);
        let mut pos = 0;

        from_fn(move || {
            if pos < self.0.len() {
                if self.0[pos * RECORD_SIZE..(pos + 2) * RECORD_SIZE]
                    .iter()
                    .all(|byte| *byte == 0)
                {
                    return None;
                }
                let header_record = HeaderRecord::from_bytes(
                    &self.0[pos * RECORD_SIZE..(pos + 1) * RECORD_SIZE]
                        .as_array()
                        .expect("Not enough bytes for header"),
                );
                let file_size = header_record.size();
                let n_file_records = file_size / RECORD_SIZE + 1;

                let bytes =
                    &self.0[(pos + 1) * RECORD_SIZE..(pos + 1 + n_file_records) * RECORD_SIZE];

                let file = File {
                    header_record,
                    bytes,
                };
                pos += 1 + n_file_records;
                Some(file)
            } else {
                None
            }
        })
    }
}

pub struct File<'a> {
    pub header_record: &'a HeaderRecord,
    pub bytes: &'a [u8],
}

#[repr(C)]
pub struct HeaderRecord {
    path: [u8; 100],
    mode: [u8; 8],
    uid: [u8; 8],
    gid: [u8; 8],
    size: [u8; 12],
    modification_time: [u8; 12],
    checksum: [u8; 8],
    link_indicator: [u8; 1],
    link_name: [u8; 100],
    _padding: [u8; 255],
}

impl HeaderRecord {
    fn from_bytes(bytes: &[u8; 512]) -> &Self {
        unsafe { &*(bytes.as_ptr() as *const Self) }
    }

    pub fn path(&self) -> &str {
        str::from_utf8(&self.path)
            .expect("Could not parse path")
            .trim_matches(char::from(0))
    }

    pub fn size(&self) -> usize {
        let str_size = str::from_utf8(&self.size).expect("Could not parse utf8");
        usize::from_str_radix(&str_size[0..11], 8).expect("Could not parse octal string")
    }
}
