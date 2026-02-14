/// Write a byte to buffer and advance position. Pure Rust, no FFI.
#[inline]
pub fn write_byte(buffer: &mut [u8], position: &mut i32, value: u8) {
    let pos = *position as usize;
    if pos < buffer.len() {
        buffer[pos] = value;
    }
    *position += 1;
}
