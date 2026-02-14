use std::slice;

extern "C" {
    pub fn quantize_block(
        block: *mut i16,
        table: *const i16,
    );
}

/// Encode grayscale image.
/// quality: 1–100 (higher = less quantization), 0 = default 85.
#[no_mangle]
pub extern "C" fn bitgrain_encode_grayscale(
    image: *const u8,
    width: u32,
    height: u32,
    out_buffer: *mut u8,
    out_capacity: u32,
    out_len: *mut i32,
    quality: u8,
) -> i32 {
    if image.is_null() || out_buffer.is_null() || out_len.is_null() || out_capacity == 0 {
        return -1;
    }
    let q = if quality == 0 { 85 } else { quality };
    let size = (width as usize).saturating_mul(height as usize);
    let image_slice = unsafe { slice::from_raw_parts(image, size) };
    let buffer_slice = unsafe { slice::from_raw_parts_mut(out_buffer, out_capacity as usize) };
    let mut pos: i32 = 0;
    crate::encoder::encode_grayscale(image_slice, width as usize, height as usize, q, buffer_slice, &mut pos);
    unsafe { *out_len = pos };
    0
}

/// Encode an RGB image (24 bpp, R G B per pixel) to .bg stream.
/// quality: 1–100, 0 = default 85.
#[no_mangle]
pub extern "C" fn bitgrain_encode_rgb(
    image: *const u8,
    width: u32,
    height: u32,
    out_buffer: *mut u8,
    out_capacity: u32,
    out_len: *mut i32,
    quality: u8,
) -> i32 {
    if image.is_null() || out_buffer.is_null() || out_len.is_null() || out_capacity == 0 {
        return -1;
    }
    let q = if quality == 0 { 85 } else { quality };
    let size = (width as usize).saturating_mul(height as usize).saturating_mul(3);
    let image_slice = unsafe { slice::from_raw_parts(image, size) };
    let buffer_slice = unsafe { slice::from_raw_parts_mut(out_buffer, out_capacity as usize) };
    let mut pos: i32 = 0;
    crate::encoder::encode_rgb(image_slice, width as usize, height as usize, q, buffer_slice, &mut pos);
    unsafe { *out_len = pos };
    0
}

/// Decode a .bg stream into pixels (grayscale or RGB per header).
/// out_channels: output, 1 = grayscale (out_pixels = w*h), 3 = RGB (out_pixels = w*h*3).
/// out_capacity debe ser >= width*height*out_channels.
#[no_mangle]
pub extern "C" fn bitgrain_decode(
    buffer: *const u8,
    size: i32,
    out_pixels: *mut u8,
    out_capacity: u32,
    out_width: *mut u32,
    out_height: *mut u32,
    out_channels: *mut u32,
) -> i32 {
    if buffer.is_null() || out_pixels.is_null() || out_width.is_null()
        || out_height.is_null() || out_channels.is_null() {
        return -1;
    }
    if size <= 0 || out_capacity == 0 {
        return -1;
    }
    let buf_slice = unsafe { slice::from_raw_parts(buffer, size as usize) };
    let out_slice = unsafe { slice::from_raw_parts_mut(out_pixels, out_capacity as usize) };
    let ok = crate::decoder::decode(
        buf_slice,
        out_slice,
        unsafe { &mut *out_width },
        unsafe { &mut *out_height },
        unsafe { &mut *out_channels },
    );
    if ok { 0 } else { -1 }
}

/// Decode a .bg stream to grayscale (version 1 .bg only).
#[no_mangle]
pub extern "C" fn bitgrain_decode_grayscale(
    buffer: *const u8,
    size: i32,
    out_pixels: *mut u8,
    out_capacity: u32,
    out_width: *mut u32,
    out_height: *mut u32,
) -> i32 {
    if buffer.is_null() || out_pixels.is_null() || out_width.is_null() || out_height.is_null() {
        return -1;
    }
    if size <= 0 || out_capacity == 0 {
        return -1;
    }
    let mut ch = 0u32;
    let buf_slice = unsafe { slice::from_raw_parts(buffer, size as usize) };
    let out_slice = unsafe { slice::from_raw_parts_mut(out_pixels, out_capacity as usize) };
    let ok = crate::decoder::decode(
        buf_slice,
        out_slice,
        unsafe { &mut *out_width },
        unsafe { &mut *out_height },
        &mut ch,
    );
    if ok && ch == 1 { 0 } else { -1 }
}
