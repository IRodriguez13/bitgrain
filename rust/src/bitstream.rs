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
    let end = pos.saturating_add(data.len());
    if end > buffer.len() 
    {
        panic!(
            "bitgrain: encode buffer exhausted (need {} bytes at pos {}, cap {})",
            data.len(),
            pos,
            buffer.len()
        );
    }
    
    if !data.is_empty() 
    {
        buffer[pos..end].copy_from_slice(data);
    }
    
    *position += data.len() as i32;
}
