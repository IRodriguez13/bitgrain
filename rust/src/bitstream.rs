/// Write a byte to buffer and advance position. Pure Rust, no FFI.
#[inline]
pub fn write_byte(buffer: &mut [u8], position: &mut i32, value: u8) {
    let pos = *position as usize;
    if pos < buffer.len() {
        buffer[pos] = value;
    }
    *position += 1;
}

/// Write bytes to buffer and advance position.
#[inline]
pub fn write_bytes(buffer: &mut [u8], position: &mut i32, data: &[u8]) {
    let pos = *position as usize;
    let n = data.len().min(buffer.len().saturating_sub(pos));
    if n > 0 {
        buffer[pos..pos + n].copy_from_slice(&data[..n]);
    }
    *position += data.len() as i32;
}
