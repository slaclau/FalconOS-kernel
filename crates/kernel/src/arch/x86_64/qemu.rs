use core::fmt;

use super::port::Port;

pub struct QemuDebugWriter;

impl fmt::Write for QemuDebugWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let mut port = Port::new_readwrite(0xe9);
        for byte in s.bytes() {
            unsafe {
                port.write(byte);
            }
        }
        Ok(())
    }
}
