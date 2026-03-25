pub fn get_bit(value: u64, bit: usize) -> u64 {
  let mask = (1 as u64) << bit;
  let masked = value & mask;
  (masked > 0) as u64
}

pub fn get_bits(value: u64, low_bit: usize, high_bit: usize) -> u64 {
  let mut ret: u64 = 0;
  for bit in low_bit..high_bit {
      ret |= ((get_bit(value, bit) as u64) << (bit - low_bit));
  }

  ret
}