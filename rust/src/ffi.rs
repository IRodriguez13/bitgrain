use std::slice;

extern "C" {
    pub fn quantize_block(
        block: *mut i16,
        table: *const i16,
    );
    pub fn bitgrain_dct_block(block: *mut i16);
    pub fn bitgrain_idct_block(block: *mut i16);
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
    crate::encoder::encode_rgb(image_slice, width as usize, height as usize, q, buffer_slice, &mut pos, None);
    unsafe { *out_len = pos };
    0
}

/// Encode an RGBA image (32 bpp, R G B A per pixel) to .bg stream.
/// quality: 1–100, 0 = default 85.
#[no_mangle]
pub extern "C" fn bitgrain_encode_rgba(
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
    let size = (width as usize).saturating_mul(height as usize).saturating_mul(4);
    let image_slice = unsafe { slice::from_raw_parts(image, size) };
    let buffer_slice = unsafe { slice::from_raw_parts_mut(out_buffer, out_capacity as usize) };
    let mut pos: i32 = 0;
    crate::encoder::encode_rgba(image_slice, width as usize, height as usize, q, buffer_slice, &mut pos, None);
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
        None,
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
        None,
    );
    if ok && ch == 1 { 0 } else { -1 }
}

/// Encode RGB with optional ICC profile.
#[no_mangle]
pub extern "C" fn bitgrain_encode_rgb_icc(
    image: *const u8,
    width: u32,
    height: u32,
    out_buffer: *mut u8,
    out_capacity: u32,
    out_len: *mut i32,
    quality: u8,
    icc: *const u8,
    icc_len: u32,
) -> i32 {
    if image.is_null() || out_buffer.is_null() || out_len.is_null() || out_capacity == 0 {
        return -1;
    }
    let q = if quality == 0 { 85 } else { quality };
    let size = (width as usize).saturating_mul(height as usize).saturating_mul(3);
    let image_slice = unsafe { slice::from_raw_parts(image, size) };
    let buffer_slice = unsafe { slice::from_raw_parts_mut(out_buffer, out_capacity as usize) };
    let mut pos: i32 = 0;
    let icc_opt = if !icc.is_null() && icc_len > 0 {
        Some(unsafe { slice::from_raw_parts(icc, icc_len as usize) })
    } else {
        None
    };
    crate::encoder::encode_rgb(image_slice, width as usize, height as usize, q, buffer_slice, &mut pos, icc_opt);
    unsafe { *out_len = pos };
    0
}

/// Encode RGBA with optional ICC profile.
#[no_mangle]
pub extern "C" fn bitgrain_encode_rgba_icc(
    image: *const u8,
    width: u32,
    height: u32,
    out_buffer: *mut u8,
    out_capacity: u32,
    out_len: *mut i32,
    quality: u8,
    icc: *const u8,
    icc_len: u32,
) -> i32 {
    if image.is_null() || out_buffer.is_null() || out_len.is_null() || out_capacity == 0 {
        return -1;
    }
    let q = if quality == 0 { 85 } else { quality };
    let size = (width as usize).saturating_mul(height as usize).saturating_mul(4);
    let image_slice = unsafe { slice::from_raw_parts(image, size) };
    let buffer_slice = unsafe { slice::from_raw_parts_mut(out_buffer, out_capacity as usize) };
    let mut pos: i32 = 0;
    let icc_opt = if !icc.is_null() && icc_len > 0 {
        Some(unsafe { slice::from_raw_parts(icc, icc_len as usize) })
    } else {
        None
    };
    crate::encoder::encode_rgba(image_slice, width as usize, height as usize, q, buffer_slice, &mut pos, icc_opt);
    unsafe { *out_len = pos };
    0
}

/// Decode .bg and optionally return embedded ICC. Caller must free ICC with bitgrain_free_icc.
#[no_mangle]
pub extern "C" fn bitgrain_decode_icc(
    buffer: *const u8,
    size: i32,
    out_pixels: *mut u8,
    out_capacity: u32,
    out_width: *mut u32,
    out_height: *mut u32,
    out_channels: *mut u32,
    out_icc: *mut *mut u8,
    out_icc_len: *mut u32,
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
    let mut icc_vec = Vec::new();
    let ok = crate::decoder::decode(
        buf_slice,
        out_slice,
        unsafe { &mut *out_width },
        unsafe { &mut *out_height },
        unsafe { &mut *out_channels },
        Some(&mut icc_vec),
    );
    if !ok {
        return -1;
    }
    if !out_icc.is_null() && !out_icc_len.is_null() {
        if icc_vec.is_empty() {
            unsafe { *out_icc = std::ptr::null_mut(); *out_icc_len = 0; }
        } else {
            let len = icc_vec.len();
            let ptr = icc_vec.as_mut_ptr();
            std::mem::forget(icc_vec);
            unsafe { *out_icc = ptr; *out_icc_len = len as u32; }
        }
    }
    0
}

/// Free ICC buffer returned by bitgrain_decode_icc.
#[no_mangle]
pub extern "C" fn bitgrain_free_icc(ptr: *mut u8, len: u32) {
    if ptr.is_null() || len == 0 {
        return;
    }
    let _ = unsafe { Vec::from_raw_parts(ptr, len as usize, len as usize) };
}
