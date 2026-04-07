use std::panic::{catch_unwind, AssertUnwindSafe};
use std::cell::{Cell, RefCell};
use std::ffi::CString;
use std::slice;
use std::sync::atomic::{AtomicUsize, Ordering};

extern "C" {
    pub fn quantize_block(
        block: *mut i16,
        table: *const i16,
    );
    pub fn dequantize_block(
        block: *mut i16,
        table: *const i16,
    );
    pub fn bitgrain_dct_block(block: *mut i16);
    pub fn bitgrain_idct_block(block: *mut i16);
}

static RAYON_THREADS_CONFIGURED: AtomicUsize = AtomicUsize::new(0);

const BITGRAIN_OK: i32 = 0;
const BITGRAIN_ERR_INVALID_ARG: i32 = 1;
const BITGRAIN_ERR_DECODE_FAILED: i32 = 2;
const BITGRAIN_ERR_THREAD_INIT: i32 = 3;
const BITGRAIN_ERR_PANIC: i32 = 100;

thread_local! {
    static LAST_ERROR_CODE: Cell<i32> = const { Cell::new(BITGRAIN_OK) };
    static LAST_ERROR_MSG: RefCell<CString> = RefCell::new(CString::new("ok").expect("static cstring"));
}

#[inline]
fn set_last_error(code: i32, msg: &'static str) {
    LAST_ERROR_CODE.with(|c| c.set(code));
    LAST_ERROR_MSG.with(|m| {
        *m.borrow_mut() = CString::new(msg).expect("static cstring");
    });
}

#[inline]
fn clear_last_error() {
    set_last_error(BITGRAIN_OK, "ok");
}

#[inline]
fn fail(code: i32, msg: &'static str) -> i32 {
    set_last_error(code, msg);
    -1
}

#[inline]
fn ffi_guard<F>(f: F) -> i32
where
    F: FnOnce() -> i32,
{
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(code) => code,
        Err(_) => fail(BITGRAIN_ERR_PANIC, "panic in codec internals"),
    }
}

#[no_mangle]
pub extern "C" fn bitgrain_last_error_code() -> i32 {
    LAST_ERROR_CODE.with(|c| c.get())
}

#[no_mangle]
pub extern "C" fn bitgrain_last_error_message() -> *const i8 {
    LAST_ERROR_MSG.with(|m| m.borrow().as_c_str().as_ptr())
}

#[no_mangle]
pub extern "C" fn bitgrain_clear_error() {
    clear_last_error();
}

/// Configure global Rayon thread pool size.
/// Must be called before any parallel codec operation starts.
/// Returns 0 on success, -1 on invalid value or late/failed init.
#[no_mangle]
pub extern "C" fn bitgrain_set_threads(threads: i32) -> i32 {
    clear_last_error();
    if threads <= 0 {
        return fail(BITGRAIN_ERR_INVALID_ARG, "threads must be > 0");
    }
    let t = threads as usize;
    let current = RAYON_THREADS_CONFIGURED.load(Ordering::Relaxed);
    if current == t {
        return 0;
    }
    if current != 0 && current != t {
        return fail(BITGRAIN_ERR_THREAD_INIT, "thread pool already configured with another value");
    }
    match rayon::ThreadPoolBuilder::new().num_threads(t).build_global() {
        Ok(()) => {
            RAYON_THREADS_CONFIGURED.store(t, Ordering::Relaxed);
            0
        }
        Err(_) => fail(BITGRAIN_ERR_THREAD_INIT, "failed to initialize global thread pool"),
    }
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
    clear_last_error();
    if image.is_null() || out_buffer.is_null() || out_len.is_null() || out_capacity == 0 {
        return fail(BITGRAIN_ERR_INVALID_ARG, "invalid encode_grayscale arguments");
    }
    ffi_guard(|| {
        let q = if quality == 0 { 85 } else { quality };
        let size = (width as usize).saturating_mul(height as usize);
        let image_slice = unsafe { slice::from_raw_parts(image, size) };
        let buffer_slice = unsafe { slice::from_raw_parts_mut(out_buffer, out_capacity as usize) };
        let mut pos: i32 = 0;
        crate::encoder::encode_grayscale(
            image_slice,
            width as usize,
            height as usize,
            q,
            buffer_slice,
            &mut pos,
        );
        unsafe { *out_len = pos };
        0
    })
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
    clear_last_error();
    if image.is_null() || out_buffer.is_null() || out_len.is_null() || out_capacity == 0 {
        return fail(BITGRAIN_ERR_INVALID_ARG, "invalid encode_rgb arguments");
    }
    ffi_guard(|| {
        let q = if quality == 0 { 85 } else { quality };
        let size = (width as usize)
            .saturating_mul(height as usize)
            .saturating_mul(3);
        let image_slice = unsafe { slice::from_raw_parts(image, size) };
        let buffer_slice = unsafe { slice::from_raw_parts_mut(out_buffer, out_capacity as usize) };
        let mut pos: i32 = 0;
        crate::encoder::encode_rgb(
            image_slice,
            width as usize,
            height as usize,
            q,
            buffer_slice,
            &mut pos,
            None,
        );
        unsafe { *out_len = pos };
        0
    })
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
    clear_last_error();
    if image.is_null() || out_buffer.is_null() || out_len.is_null() || out_capacity == 0 {
        return fail(BITGRAIN_ERR_INVALID_ARG, "invalid encode_rgba arguments");
    }
    ffi_guard(|| {
        let q = if quality == 0 { 85 } else { quality };
        let size = (width as usize)
            .saturating_mul(height as usize)
            .saturating_mul(4);
        let image_slice = unsafe { slice::from_raw_parts(image, size) };
        let buffer_slice = unsafe { slice::from_raw_parts_mut(out_buffer, out_capacity as usize) };
        let mut pos: i32 = 0;
        crate::encoder::encode_rgba(
            image_slice,
            width as usize,
            height as usize,
            q,
            buffer_slice,
            &mut pos,
            None,
        );
        unsafe { *out_len = pos };
        0
    })
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
    clear_last_error();
    if buffer.is_null() || out_pixels.is_null() || out_width.is_null()
        || out_height.is_null() || out_channels.is_null() {
        return fail(BITGRAIN_ERR_INVALID_ARG, "invalid decode arguments");
    }
    if size <= 0 || out_capacity == 0 {
        return fail(BITGRAIN_ERR_INVALID_ARG, "invalid decode buffer size/capacity");
    }
    ffi_guard(|| {
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
        if ok {
            0
        } else {
            fail(BITGRAIN_ERR_DECODE_FAILED, "decode failed")
        }
    })
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
    clear_last_error();
    if buffer.is_null() || out_pixels.is_null() || out_width.is_null() || out_height.is_null() {
        return fail(BITGRAIN_ERR_INVALID_ARG, "invalid decode_grayscale arguments");
    }
    if size <= 0 || out_capacity == 0 {
        return fail(BITGRAIN_ERR_INVALID_ARG, "invalid decode_grayscale buffer size/capacity");
    }
    ffi_guard(|| {
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
        if ok && ch == 1 {
            0
        } else {
            fail(BITGRAIN_ERR_DECODE_FAILED, "decode_grayscale failed or output is not grayscale")
        }
    })
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
    clear_last_error();
    if image.is_null() || out_buffer.is_null() || out_len.is_null() || out_capacity == 0 {
        return fail(BITGRAIN_ERR_INVALID_ARG, "invalid encode_rgb_icc arguments");
    }
    ffi_guard(|| {
        let q = if quality == 0 { 85 } else { quality };
        let size = (width as usize)
            .saturating_mul(height as usize)
            .saturating_mul(3);
        let image_slice = unsafe { slice::from_raw_parts(image, size) };
        let buffer_slice = unsafe { slice::from_raw_parts_mut(out_buffer, out_capacity as usize) };
        let mut pos: i32 = 0;
        let icc_opt = if !icc.is_null() && icc_len > 0 {
            Some(unsafe { slice::from_raw_parts(icc, icc_len as usize) })
        } else {
            None
        };
        crate::encoder::encode_rgb(
            image_slice,
            width as usize,
            height as usize,
            q,
            buffer_slice,
            &mut pos,
            icc_opt,
        );
        unsafe { *out_len = pos };
        0
    })
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
    clear_last_error();
    if image.is_null() || out_buffer.is_null() || out_len.is_null() || out_capacity == 0 {
        return fail(BITGRAIN_ERR_INVALID_ARG, "invalid encode_rgba_icc arguments");
    }
    ffi_guard(|| {
        let q = if quality == 0 { 85 } else { quality };
        let size = (width as usize)
            .saturating_mul(height as usize)
            .saturating_mul(4);
        let image_slice = unsafe { slice::from_raw_parts(image, size) };
        let buffer_slice = unsafe { slice::from_raw_parts_mut(out_buffer, out_capacity as usize) };
        let mut pos: i32 = 0;
        let icc_opt = if !icc.is_null() && icc_len > 0 {
            Some(unsafe { slice::from_raw_parts(icc, icc_len as usize) })
        } else {
            None
        };
        crate::encoder::encode_rgba(
            image_slice,
            width as usize,
            height as usize,
            q,
            buffer_slice,
            &mut pos,
            icc_opt,
        );
        unsafe { *out_len = pos };
        0
    })
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
    clear_last_error();
    if buffer.is_null() || out_pixels.is_null() || out_width.is_null()
        || out_height.is_null() || out_channels.is_null() {
        return fail(BITGRAIN_ERR_INVALID_ARG, "invalid decode_icc arguments");
    }
    if size <= 0 || out_capacity == 0 {
        return fail(BITGRAIN_ERR_INVALID_ARG, "invalid decode_icc buffer size/capacity");
    }
    ffi_guard(|| {
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
            return fail(BITGRAIN_ERR_DECODE_FAILED, "decode_icc failed");
        }
        if !out_icc.is_null() && !out_icc_len.is_null() {
            if icc_vec.is_empty() {
                unsafe {
                    *out_icc = std::ptr::null_mut();
                    *out_icc_len = 0;
                }
            } else {
                let len = icc_vec.len();
                let ptr = icc_vec.as_mut_ptr();
                std::mem::forget(icc_vec);
                unsafe {
                    *out_icc = ptr;
                    *out_icc_len = len as u32;
                }
            }
        }
        0
    })
}

/// Free ICC buffer returned by bitgrain_decode_icc.
#[no_mangle]
pub extern "C" fn bitgrain_free_icc(ptr: *mut u8, len: u32) {
    if ptr.is_null() || len == 0 {
        return;
    }
    let _ = unsafe { Vec::from_raw_parts(ptr, len as usize, len as usize) };
}
