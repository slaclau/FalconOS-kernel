#![no_std]

use core::iter::from_fn;

pub struct Archive<'a>(pub &'a [u8]);

impl<'a> Archive<'a> {
    pub fn files(&self) -> impl Iterator<Item = File<'a>> {
        let (records, remainder) = self.0.as_chunks::<512>();
        assert!(remainder.len() == 0);
        let mut pos = 0;
        from_fn(move || {
            if pos < records.len() {
                if records[pos..pos + 2]
                    .iter()
                    .all(|record| record.iter().all(|byte| *byte == 0))
                {
                    return None;
                }
                let header_record = HeaderRecord::from_bytes(&records[pos]);
                let file_size = header_record.size();
                let n_file_records = file_size / 512 + 1;

                let file_records = &records[pos + 1..pos + 1 + n_file_records];

                let file = File {
                    header_record,
                    file_records,
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
    file_records: &'a [[u8; 512]],
}

impl<'a> File<'a> {
    pub fn n_file_records(&self) -> usize {
        self.file_records.len()
    }
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
        str::from_utf8(&self.path).expect("Could not parse path")
    }

    pub fn size(&self) -> usize {
        let str_size = str::from_utf8(&self.size).expect("Could not parse utf8");
        usize::from_str_radix(&str_size[0..11], 8).expect("Could not parse octal string")
    }
}
