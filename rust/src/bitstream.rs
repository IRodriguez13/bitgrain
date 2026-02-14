use crate::ffi::bitstream_write_byte;

pub fn write_byte(
    buffer: &mut [u8],
    position: &mut i32,
    value: u8,
) {
    unsafe {
        bitstream_write_byte(
            buffer.as_mut_ptr(),
            position as *mut i32,
            value,
        );
    }
}

