pub mod ring_buffer;
pub mod bits;

#[macro_export]
macro_rules! log {
    ($buffer:ident, $($args:tt)*) => {
      let mut str_buffer = [0 as u8; $crate::utils::ring_buffer::RING_BUFFER_ENTRY_SIZE];
      let mut ring_buffer_entry = $crate::utils::ring_buffer::RingBufferEntryWrapper::new(&mut str_buffer);
      ring_buffer_entry.write_fmt(format_args!($($args)*)).expect("");
      $buffer.lock().write_str(ring_buffer_entry.as_str()).expect("");
    };
}
