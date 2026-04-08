use core::fmt::{self, Debug, Error, Write};

pub const RING_BUFFER_LENGTH: usize = 1024;
pub const RING_BUFFER_ENTRY_SIZE: usize = 128;

pub struct RingBufferEntryWrapper<'a> {
    entry: &'a mut [u8; RING_BUFFER_ENTRY_SIZE],
    offset: usize,
}

impl<'a> RingBufferEntryWrapper<'a> {
    pub fn new(buf: &'a mut [u8; RING_BUFFER_ENTRY_SIZE]) -> Self {
        RingBufferEntryWrapper {
            entry: buf,
            offset: 0,
        }
    }

    pub fn as_str(&self) -> &str {
        str::from_utf8(self.entry).expect("should be a string")
    }
}

impl<'a> fmt::Write for RingBufferEntryWrapper<'a> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let mut bytes = s.as_bytes();

        // Skip over already-copied data
        let remainder = &mut self.entry[self.offset..];
        // Check if there is space remaining (return error instead of panicking)
        if remainder.len() < bytes.len() {
            bytes = &bytes[0..remainder.len()]
        }
        // Make the two slices the same length
        let remainder = &mut remainder[..bytes.len()];
        // Copy
        remainder.copy_from_slice(bytes);

        // Update offset to avoid overwriting
        self.offset += bytes.len();

        Ok(())
    }
}

pub struct RingBuffer<const LENGTH: usize> {
    start: usize,
    length: usize,
    size: usize,
    entries: [[u8; RING_BUFFER_ENTRY_SIZE]; LENGTH],
}

impl<const LENGTH: usize> RingBuffer<LENGTH> {
    pub const fn new() -> Self {
        Self {
            start: 0,
            length: 0,
            size: LENGTH,
            entries: [[0; RING_BUFFER_ENTRY_SIZE]; LENGTH],
        }
    }

    pub fn is_empty(&self) -> bool {
        self.length == 0
    }

    pub fn is_full(&self) -> bool {
        self.length >= self.size
    }

    pub fn read_str(&mut self, buffer: &mut [u8; RING_BUFFER_ENTRY_SIZE]) -> Result<(), ()> {
        if self.is_empty() {
            return Err(());
        }
        let entry = self.entries[self.start];

        buffer.clone_from_slice(&entry);
        self.entries[self.start] = [0; RING_BUFFER_ENTRY_SIZE];
        self.start += 1;
        self.length -= 1;
        Ok(())
    }

    pub fn write_str(&mut self, msg: &str) -> Result<(), Error> {
        if msg.len() > RING_BUFFER_ENTRY_SIZE {
            return Err(Error);
        }
        let mut entry = [0; RING_BUFFER_ENTRY_SIZE];
        for (index, byte) in msg.bytes().enumerate() {
            entry[index] = byte;
        }
        let index = (self.start + self.length) % self.size;
        self.entries[index] = entry;
        self.length += 1;
        Ok(())
    }

    pub fn dump(&mut self, writer: impl Write) {
        self.dump_with_reason("", writer);
    }

    pub fn dump_with_reason(&mut self, reason: &str, mut writer: impl Write) {
        if !reason.is_empty() {
            writer
                .write_fmt(format_args!("Dumping ring buffer ({reason}): {self:?}\n"))
                .expect("Failed to write to writer");
        } else {
            writer
                .write_fmt(format_args!("Dumping ring buffer: {self:?}\n"))
                .expect("Failed to write to writer");
        }
        let mut buffer = [0_u8; RING_BUFFER_ENTRY_SIZE];

        let mut i = 0;

        while !self.is_empty() {
            self.read_str(&mut buffer)
                .expect("Failed to read from ring buffer");
            let msg = str::from_utf8(&buffer).expect("Failed to parse buffer");
            writer
                .write_fmt(format_args!("ENTRY {i}: {msg}\n"))
                .expect("Failed to write to VGA Buffer");
            i += 1;
        }
    }
}

impl<const LENGTH: usize> Debug for RingBuffer<LENGTH> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RingBuffer")
            .field("start", &self.start)
            .field("size", &self.size)
            .field("length", &self.length)
            .field_with("address", |f| f.write_fmt(format_args!("{self:p}")))
            .finish()
    }
}
