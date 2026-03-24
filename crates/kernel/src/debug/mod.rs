#[cfg(target_arch = "x86_64")]
mod vga_buffer;
pub use vga_buffer::make_writer;
pub use vga_buffer::Writer;