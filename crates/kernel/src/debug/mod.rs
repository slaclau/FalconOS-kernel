#[cfg(target_arch = "x86_64")]
mod vga_buffer;
#[allow(unused)]
pub use vga_buffer::make_writer;
