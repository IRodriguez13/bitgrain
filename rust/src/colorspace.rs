//! RGB ↔ YCbCr color space conversion and 4:2:0 chroma subsampling.
//!
//! Uses BT.601 full-range coefficients (same as JPEG):
//!   Y  =  0.299·R + 0.587·G + 0.114·B
//!   Cb = -0.168736·R - 0.331264·G + 0.5·B + 128
//!   Cr =  0.5·R - 0.418688·G - 0.081312·B + 128
//!
//! 4:2:0 subsampling: Cb and Cr planes are downsampled to (ceil(W/2)) × (ceil(H/2))
//! using a simple 2×2 box average, matching JPEG's default chroma subsampling.
use rayon::prelude::*;
const PARALLEL_DECODE_PIXELS_THRESHOLD: usize = 262_144; // ~512x512
const PARALLEL_ENCODE_PIXELS_THRESHOLD: usize = 262_144; // ~512x512
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
use std::arch::is_x86_feature_detected;
#[cfg(target_arch = "arm")]
use std::arch::is_arm_feature_detected;
#[cfg(target_arch = "x86")]
use std::arch::x86::*;
#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;
#[cfg(target_arch = "arm")]
use std::arch::arm::*;
#[cfg(target_arch = "aarch64")]
use std::arch::aarch64::*;
#[inline]
fn clamp_u8(v: i32) -> u8 {
    if v < 0 { 0 } else if v > 255 { 255 } else { v as u8 }
}

#[inline]
fn interleave_rgb_planar(r: &[u8], g: &[u8], b: &[u8], out_row: &mut [u8]) {
    debug_assert_eq!(r.len(), g.len());
    debug_assert_eq!(r.len(), b.len());
    let mut i = 0usize;
    let mut o = 0usize;
    while i + 3 < r.len() {
        out_row[o] = r[i];
        out_row[o + 1] = g[i];
        out_row[o + 2] = b[i];
        out_row[o + 3] = r[i + 1];
        out_row[o + 4] = g[i + 1];
        out_row[o + 5] = b[i + 1];
        out_row[o + 6] = r[i + 2];
        out_row[o + 7] = g[i + 2];
        out_row[o + 8] = b[i + 2];
        out_row[o + 9] = r[i + 3];
        out_row[o + 10] = g[i + 3];
        out_row[o + 11] = b[i + 3];
        i += 4;
        o += 12;
    }
    while i < r.len() {
        out_row[o] = r[i];
        out_row[o + 1] = g[i];
        out_row[o + 2] = b[i];
        i += 1;
        o += 3;
    }
}

#[inline]
fn interleave_rgba_planar(r: &[u8], g: &[u8], b: &[u8], a: &[u8], out_row: &mut [u8]) {
    debug_assert_eq!(r.len(), g.len());
    debug_assert_eq!(r.len(), b.len());
    debug_assert_eq!(r.len(), a.len());
    let mut i = 0usize;
    let mut o = 0usize;
    while i + 3 < r.len() {
        out_row[o] = r[i];
        out_row[o + 1] = g[i];
        out_row[o + 2] = b[i];
        out_row[o + 3] = a[i];
        out_row[o + 4] = r[i + 1];
        out_row[o + 5] = g[i + 1];
        out_row[o + 6] = b[i + 1];
        out_row[o + 7] = a[i + 1];
        out_row[o + 8] = r[i + 2];
        out_row[o + 9] = g[i + 2];
        out_row[o + 10] = b[i + 2];
        out_row[o + 11] = a[i + 2];
        out_row[o + 12] = r[i + 3];
        out_row[o + 13] = g[i + 3];
        out_row[o + 14] = b[i + 3];
        out_row[o + 15] = a[i + 3];
        i += 4;
        o += 16;
    }
    while i < r.len() {
        out_row[o] = r[i];
        out_row[o + 1] = g[i];
        out_row[o + 2] = b[i];
        out_row[o + 3] = a[i];
        i += 1;
        o += 4;
    }
}

#[inline]
fn ycbcr420_to_rgb_row_scalar(
    y_row: &[u8],
    cb_row: &[u8],
    cr_row: &[u8],
    w: usize,
    out_row: &mut [u8],
) {
    let mut px = 0usize;
    while px + 1 < w {
        let cbi = cb_row[px >> 1] as i32 - 128;
        let cri = cr_row[px >> 1] as i32 - 128;
        let r_add = (359 * cri) >> 8;
        let g_sub = (88 * cbi + 183 * cri) >> 8;
        let b_add = (454 * cbi) >> 8;

        let yi0 = y_row[px] as i32;
        let o0 = px * 3;
        out_row[o0] = clamp_u8(yi0 + r_add);
        out_row[o0 + 1] = clamp_u8(yi0 - g_sub);
        out_row[o0 + 2] = clamp_u8(yi0 + b_add);

        let yi1 = y_row[px + 1] as i32;
        let o1 = o0 + 3;
        out_row[o1] = clamp_u8(yi1 + r_add);
        out_row[o1 + 1] = clamp_u8(yi1 - g_sub);
        out_row[o1 + 2] = clamp_u8(yi1 + b_add);

        px += 2;
    }
    if px < w {
        let cbi = cb_row[px >> 1] as i32 - 128;
        let cri = cr_row[px >> 1] as i32 - 128;
        let yi = y_row[px] as i32;
        let i = px * 3;
        out_row[i] = clamp_u8(yi + ((359 * cri) >> 8));
        out_row[i + 1] = clamp_u8(yi - ((88 * cbi + 183 * cri) >> 8));
        out_row[i + 2] = clamp_u8(yi + ((454 * cbi) >> 8));
    }
}

#[inline]
fn ycbcr420a_to_rgba_row_scalar(
    y_row: &[u8],
    cb_row: &[u8],
    cr_row: &[u8],
    a_row: &[u8],
    w: usize,
    out_row: &mut [u8],
) {
    let mut px = 0usize;
    while px + 1 < w {
        let cbi = cb_row[px >> 1] as i32 - 128;
        let cri = cr_row[px >> 1] as i32 - 128;
        let r_add = (359 * cri) >> 8;
        let g_sub = (88 * cbi + 183 * cri) >> 8;
        let b_add = (454 * cbi) >> 8;

        let yi0 = y_row[px] as i32;
        let o0 = px * 4;
        out_row[o0] = clamp_u8(yi0 + r_add);
        out_row[o0 + 1] = clamp_u8(yi0 - g_sub);
        out_row[o0 + 2] = clamp_u8(yi0 + b_add);
        out_row[o0 + 3] = a_row[px];

        let yi1 = y_row[px + 1] as i32;
        let o1 = o0 + 4;
        out_row[o1] = clamp_u8(yi1 + r_add);
        out_row[o1 + 1] = clamp_u8(yi1 - g_sub);
        out_row[o1 + 2] = clamp_u8(yi1 + b_add);
        out_row[o1 + 3] = a_row[px + 1];

        px += 2;
    }
    if px < w {
        let cbi = cb_row[px >> 1] as i32 - 128;
        let cri = cr_row[px >> 1] as i32 - 128;
        let yi = y_row[px] as i32;
        let i = px * 4;
        out_row[i] = clamp_u8(yi + ((359 * cri) >> 8));
        out_row[i + 1] = clamp_u8(yi - ((88 * cbi + 183 * cri) >> 8));
        out_row[i + 2] = clamp_u8(yi + ((454 * cbi) >> 8));
        out_row[i + 3] = a_row[px];
    }
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[target_feature(enable = "sse2")]
unsafe fn ycbcr420_to_rgb_row_sse2(
    y_row: &[u8],
    cb_row: &[u8],
    cr_row: &[u8],
    w: usize,
    out_row: &mut [u8],
) {
    let zero = _mm_setzero_si128();
    let mut px = 0usize;
    let mut r_add_arr = [0i16; 16];
    let mut g_sub_arr = [0i16; 16];
    let mut b_add_arr = [0i16; 16];
    let mut rv = [0u8; 16];
    let mut gv = [0u8; 16];
    let mut bv = [0u8; 16];
    while px + 16 <= w {
        for pair in 0..8usize {
            let cbi = cb_row[(px >> 1) + pair] as i32 - 128;
            let cri = cr_row[(px >> 1) + pair] as i32 - 128;
            let r_add = ((359 * cri) >> 8) as i16;
            let g_sub = ((88 * cbi + 183 * cri) >> 8) as i16;
            let b_add = ((454 * cbi) >> 8) as i16;
            let i = pair * 2;
            r_add_arr[i] = r_add;
            r_add_arr[i + 1] = r_add;
            g_sub_arr[i] = g_sub;
            g_sub_arr[i + 1] = g_sub;
            b_add_arr[i] = b_add;
            b_add_arr[i + 1] = b_add;
        }

        let yv = _mm_loadu_si128(y_row.as_ptr().add(px) as *const __m128i);
        let y_lo = _mm_unpacklo_epi8(yv, zero);
        let y_hi = _mm_unpackhi_epi8(yv, zero);

        let r_add_lo = _mm_loadu_si128(r_add_arr.as_ptr() as *const __m128i);
        let r_add_hi = _mm_loadu_si128(r_add_arr.as_ptr().add(8) as *const __m128i);
        let g_sub_lo = _mm_loadu_si128(g_sub_arr.as_ptr() as *const __m128i);
        let g_sub_hi = _mm_loadu_si128(g_sub_arr.as_ptr().add(8) as *const __m128i);
        let b_add_lo = _mm_loadu_si128(b_add_arr.as_ptr() as *const __m128i);
        let b_add_hi = _mm_loadu_si128(b_add_arr.as_ptr().add(8) as *const __m128i);

        let r_lo = _mm_add_epi16(y_lo, r_add_lo);
        let r_hi = _mm_add_epi16(y_hi, r_add_hi);
        let g_lo = _mm_sub_epi16(y_lo, g_sub_lo);
        let g_hi = _mm_sub_epi16(y_hi, g_sub_hi);
        let b_lo = _mm_add_epi16(y_lo, b_add_lo);
        let b_hi = _mm_add_epi16(y_hi, b_add_hi);

        let r8 = _mm_packus_epi16(r_lo, r_hi);
        let g8 = _mm_packus_epi16(g_lo, g_hi);
        let b8 = _mm_packus_epi16(b_lo, b_hi);

        _mm_storeu_si128(rv.as_mut_ptr() as *mut __m128i, r8);
        _mm_storeu_si128(gv.as_mut_ptr() as *mut __m128i, g8);
        _mm_storeu_si128(bv.as_mut_ptr() as *mut __m128i, b8);
        interleave_rgb_planar(&rv, &gv, &bv, &mut out_row[px * 3..(px + 16) * 3]);
        px += 16;
    }

    if px < w {
        ycbcr420_to_rgb_row_scalar(&y_row[px..], &cb_row[px >> 1..], &cr_row[px >> 1..], w - px, &mut out_row[px * 3..]);
    }
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[target_feature(enable = "avx2")]
unsafe fn ycbcr420_to_rgb_row_avx2(
    y_row: &[u8],
    cb_row: &[u8],
    cr_row: &[u8],
    w: usize,
    out_row: &mut [u8],
) {
    let zero = _mm256_setzero_si256();
    let mut px = 0usize;
    let mut r_add_arr = [0i16; 32];
    let mut g_sub_arr = [0i16; 32];
    let mut b_add_arr = [0i16; 32];
    let mut rv = [0u8; 32];
    let mut gv = [0u8; 32];
    let mut bv = [0u8; 32];
    while px + 32 <= w {
        for pair in 0..16usize {
            let cbi = cb_row[(px >> 1) + pair] as i32 - 128;
            let cri = cr_row[(px >> 1) + pair] as i32 - 128;
            let r_add = ((359 * cri) >> 8) as i16;
            let g_sub = ((88 * cbi + 183 * cri) >> 8) as i16;
            let b_add = ((454 * cbi) >> 8) as i16;
            let i = pair * 2;
            r_add_arr[i] = r_add;
            r_add_arr[i + 1] = r_add;
            g_sub_arr[i] = g_sub;
            g_sub_arr[i + 1] = g_sub;
            b_add_arr[i] = b_add;
            b_add_arr[i + 1] = b_add;
        }

        let yv = _mm256_loadu_si256(y_row.as_ptr().add(px) as *const __m256i);
        let y_lo = _mm256_unpacklo_epi8(yv, zero);
        let y_hi = _mm256_unpackhi_epi8(yv, zero);

        let r_add_lo = _mm256_loadu_si256(r_add_arr.as_ptr() as *const __m256i);
        let r_add_hi = _mm256_loadu_si256(r_add_arr.as_ptr().add(16) as *const __m256i);
        let g_sub_lo = _mm256_loadu_si256(g_sub_arr.as_ptr() as *const __m256i);
        let g_sub_hi = _mm256_loadu_si256(g_sub_arr.as_ptr().add(16) as *const __m256i);
        let b_add_lo = _mm256_loadu_si256(b_add_arr.as_ptr() as *const __m256i);
        let b_add_hi = _mm256_loadu_si256(b_add_arr.as_ptr().add(16) as *const __m256i);

        let r_lo = _mm256_add_epi16(y_lo, r_add_lo);
        let r_hi = _mm256_add_epi16(y_hi, r_add_hi);
        let g_lo = _mm256_sub_epi16(y_lo, g_sub_lo);
        let g_hi = _mm256_sub_epi16(y_hi, g_sub_hi);
        let b_lo = _mm256_add_epi16(y_lo, b_add_lo);
        let b_hi = _mm256_add_epi16(y_hi, b_add_hi);

        let r8 = _mm256_packus_epi16(r_lo, r_hi);
        let g8 = _mm256_packus_epi16(g_lo, g_hi);
        let b8 = _mm256_packus_epi16(b_lo, b_hi);

        _mm256_storeu_si256(rv.as_mut_ptr() as *mut __m256i, r8);
        _mm256_storeu_si256(gv.as_mut_ptr() as *mut __m256i, g8);
        _mm256_storeu_si256(bv.as_mut_ptr() as *mut __m256i, b8);
        interleave_rgb_planar(&rv, &gv, &bv, &mut out_row[px * 3..(px + 32) * 3]);
        px += 32;
    }
    if px < w {
        ycbcr420_to_rgb_row_sse2(
            &y_row[px..],
            &cb_row[px >> 1..],
            &cr_row[px >> 1..],
            w - px,
            &mut out_row[px * 3..],
        );
    }
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[target_feature(enable = "sse2")]
unsafe fn ycbcr420a_to_rgba_row_sse2(
    y_row: &[u8],
    cb_row: &[u8],
    cr_row: &[u8],
    a_row: &[u8],
    w: usize,
    out_row: &mut [u8],
) {
    let zero = _mm_setzero_si128();
    let mut px = 0usize;
    let mut r_add_arr = [0i16; 16];
    let mut g_sub_arr = [0i16; 16];
    let mut b_add_arr = [0i16; 16];
    let mut rv = [0u8; 16];
    let mut gv = [0u8; 16];
    let mut bv = [0u8; 16];
    let mut av = [0u8; 16];
    while px + 16 <= w {
        for pair in 0..8usize {
            let cbi = cb_row[(px >> 1) + pair] as i32 - 128;
            let cri = cr_row[(px >> 1) + pair] as i32 - 128;
            let r_add = ((359 * cri) >> 8) as i16;
            let g_sub = ((88 * cbi + 183 * cri) >> 8) as i16;
            let b_add = ((454 * cbi) >> 8) as i16;
            let i = pair * 2;
            r_add_arr[i] = r_add;
            r_add_arr[i + 1] = r_add;
            g_sub_arr[i] = g_sub;
            g_sub_arr[i + 1] = g_sub;
            b_add_arr[i] = b_add;
            b_add_arr[i + 1] = b_add;
        }

        let yv = _mm_loadu_si128(y_row.as_ptr().add(px) as *const __m128i);
        let y_lo = _mm_unpacklo_epi8(yv, zero);
        let y_hi = _mm_unpackhi_epi8(yv, zero);

        let r_add_lo = _mm_loadu_si128(r_add_arr.as_ptr() as *const __m128i);
        let r_add_hi = _mm_loadu_si128(r_add_arr.as_ptr().add(8) as *const __m128i);
        let g_sub_lo = _mm_loadu_si128(g_sub_arr.as_ptr() as *const __m128i);
        let g_sub_hi = _mm_loadu_si128(g_sub_arr.as_ptr().add(8) as *const __m128i);
        let b_add_lo = _mm_loadu_si128(b_add_arr.as_ptr() as *const __m128i);
        let b_add_hi = _mm_loadu_si128(b_add_arr.as_ptr().add(8) as *const __m128i);

        let r_lo = _mm_add_epi16(y_lo, r_add_lo);
        let r_hi = _mm_add_epi16(y_hi, r_add_hi);
        let g_lo = _mm_sub_epi16(y_lo, g_sub_lo);
        let g_hi = _mm_sub_epi16(y_hi, g_sub_hi);
        let b_lo = _mm_add_epi16(y_lo, b_add_lo);
        let b_hi = _mm_add_epi16(y_hi, b_add_hi);

        let r8 = _mm_packus_epi16(r_lo, r_hi);
        let g8 = _mm_packus_epi16(g_lo, g_hi);
        let b8 = _mm_packus_epi16(b_lo, b_hi);
        let a8 = _mm_loadu_si128(a_row.as_ptr().add(px) as *const __m128i);

        _mm_storeu_si128(rv.as_mut_ptr() as *mut __m128i, r8);
        _mm_storeu_si128(gv.as_mut_ptr() as *mut __m128i, g8);
        _mm_storeu_si128(bv.as_mut_ptr() as *mut __m128i, b8);
        _mm_storeu_si128(av.as_mut_ptr() as *mut __m128i, a8);
        interleave_rgba_planar(&rv, &gv, &bv, &av, &mut out_row[px * 4..(px + 16) * 4]);
        px += 16;
    }

    if px < w {
        ycbcr420a_to_rgba_row_scalar(
            &y_row[px..],
            &cb_row[px >> 1..],
            &cr_row[px >> 1..],
            &a_row[px..],
            w - px,
            &mut out_row[px * 4..],
        );
    }
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[target_feature(enable = "avx2")]
unsafe fn ycbcr420a_to_rgba_row_avx2(
    y_row: &[u8],
    cb_row: &[u8],
    cr_row: &[u8],
    a_row: &[u8],
    w: usize,
    out_row: &mut [u8],
) {
    let zero = _mm256_setzero_si256();
    let mut px = 0usize;
    let mut r_add_arr = [0i16; 32];
    let mut g_sub_arr = [0i16; 32];
    let mut b_add_arr = [0i16; 32];
    let mut rv = [0u8; 32];
    let mut gv = [0u8; 32];
    let mut bv = [0u8; 32];
    let mut av = [0u8; 32];
    while px + 32 <= w {
        for pair in 0..16usize {
            let cbi = cb_row[(px >> 1) + pair] as i32 - 128;
            let cri = cr_row[(px >> 1) + pair] as i32 - 128;
            let r_add = ((359 * cri) >> 8) as i16;
            let g_sub = ((88 * cbi + 183 * cri) >> 8) as i16;
            let b_add = ((454 * cbi) >> 8) as i16;
            let i = pair * 2;
            r_add_arr[i] = r_add;
            r_add_arr[i + 1] = r_add;
            g_sub_arr[i] = g_sub;
            g_sub_arr[i + 1] = g_sub;
            b_add_arr[i] = b_add;
            b_add_arr[i + 1] = b_add;
        }

        let yv = _mm256_loadu_si256(y_row.as_ptr().add(px) as *const __m256i);
        let y_lo = _mm256_unpacklo_epi8(yv, zero);
        let y_hi = _mm256_unpackhi_epi8(yv, zero);

        let r_add_lo = _mm256_loadu_si256(r_add_arr.as_ptr() as *const __m256i);
        let r_add_hi = _mm256_loadu_si256(r_add_arr.as_ptr().add(16) as *const __m256i);
        let g_sub_lo = _mm256_loadu_si256(g_sub_arr.as_ptr() as *const __m256i);
        let g_sub_hi = _mm256_loadu_si256(g_sub_arr.as_ptr().add(16) as *const __m256i);
        let b_add_lo = _mm256_loadu_si256(b_add_arr.as_ptr() as *const __m256i);
        let b_add_hi = _mm256_loadu_si256(b_add_arr.as_ptr().add(16) as *const __m256i);

        let r_lo = _mm256_add_epi16(y_lo, r_add_lo);
        let r_hi = _mm256_add_epi16(y_hi, r_add_hi);
        let g_lo = _mm256_sub_epi16(y_lo, g_sub_lo);
        let g_hi = _mm256_sub_epi16(y_hi, g_sub_hi);
        let b_lo = _mm256_add_epi16(y_lo, b_add_lo);
        let b_hi = _mm256_add_epi16(y_hi, b_add_hi);

        let r8 = _mm256_packus_epi16(r_lo, r_hi);
        let g8 = _mm256_packus_epi16(g_lo, g_hi);
        let b8 = _mm256_packus_epi16(b_lo, b_hi);
        let a8 = _mm256_loadu_si256(a_row.as_ptr().add(px) as *const __m256i);

        _mm256_storeu_si256(rv.as_mut_ptr() as *mut __m256i, r8);
        _mm256_storeu_si256(gv.as_mut_ptr() as *mut __m256i, g8);
        _mm256_storeu_si256(bv.as_mut_ptr() as *mut __m256i, b8);
        _mm256_storeu_si256(av.as_mut_ptr() as *mut __m256i, a8);
        interleave_rgba_planar(&rv, &gv, &bv, &av, &mut out_row[px * 4..(px + 32) * 4]);
        px += 32;
    }
    if px < w {
        ycbcr420a_to_rgba_row_sse2(
            &y_row[px..],
            &cb_row[px >> 1..],
            &cr_row[px >> 1..],
            &a_row[px..],
            w - px,
            &mut out_row[px * 4..],
        );
    }
}

#[cfg(any(target_arch = "arm", target_arch = "aarch64"))]
#[inline]
#[target_feature(enable = "neon")]
unsafe fn ycbcr_to_rgb8_neon(y8: uint8x8_t, cb8: uint8x8_t, cr8: uint8x8_t) -> (uint8x8_t, uint8x8_t, uint8x8_t) {
    let y16 = vreinterpretq_s16_u16(vmovl_u8(y8));
    let d16 = vsubq_s16(vreinterpretq_s16_u16(vmovl_u8(cb8)), vdupq_n_s16(128));
    let e16 = vsubq_s16(vreinterpretq_s16_u16(vmovl_u8(cr8)), vdupq_n_s16(128));

    let yl = vmovl_s16(vget_low_s16(y16));
    let yh = vmovl_s16(vget_high_s16(y16));
    let dl = vmovl_s16(vget_low_s16(d16));
    let dh = vmovl_s16(vget_high_s16(d16));
    let el = vmovl_s16(vget_low_s16(e16));
    let eh = vmovl_s16(vget_high_s16(e16));

    let r_lo = vaddq_s32(
        yl,
        vshrq_n_s32(vaddq_s32(vmulq_n_s32(el, 359), vdupq_n_s32(128)), 8),
    );
    let r_hi = vaddq_s32(
        yh,
        vshrq_n_s32(vaddq_s32(vmulq_n_s32(eh, 359), vdupq_n_s32(128)), 8),
    );
    let g_lo = vsubq_s32(
        yl,
        vshrq_n_s32(
            vaddq_s32(
                vaddq_s32(vmulq_n_s32(dl, 88), vmulq_n_s32(el, 183)),
                vdupq_n_s32(128),
            ),
            8,
        ),
    );
    let g_hi = vsubq_s32(
        yh,
        vshrq_n_s32(
            vaddq_s32(
                vaddq_s32(vmulq_n_s32(dh, 88), vmulq_n_s32(eh, 183)),
                vdupq_n_s32(128),
            ),
            8,
        ),
    );
    let b_lo = vaddq_s32(
        yl,
        vshrq_n_s32(vaddq_s32(vmulq_n_s32(dl, 454), vdupq_n_s32(128)), 8),
    );
    let b_hi = vaddq_s32(
        yh,
        vshrq_n_s32(vaddq_s32(vmulq_n_s32(dh, 454), vdupq_n_s32(128)), 8),
    );

    let r16 = vcombine_s16(vqmovn_s32(r_lo), vqmovn_s32(r_hi));
    let g16 = vcombine_s16(vqmovn_s32(g_lo), vqmovn_s32(g_hi));
    let b16 = vcombine_s16(vqmovn_s32(b_lo), vqmovn_s32(b_hi));

    (vqmovun_s16(r16), vqmovun_s16(g16), vqmovun_s16(b16))
}

#[cfg(any(target_arch = "arm", target_arch = "aarch64"))]
#[inline]
#[target_feature(enable = "neon")]
unsafe fn ycbcr420_to_rgb_row_neon(
    y_row: &[u8],
    cb_row: &[u8],
    cr_row: &[u8],
    w: usize,
    out_row: &mut [u8],
) {
    let mut px = 0usize;
    let mut rv = [0u8; 16];
    let mut gv = [0u8; 16];
    let mut bv = [0u8; 16];
    while px + 16 <= w {
        let cb8 = vld1_u8(cb_row.as_ptr().add(px >> 1));
        let cr8 = vld1_u8(cr_row.as_ptr().add(px >> 1));
        let cbz = vzip_u8(cb8, cb8);
        let crz = vzip_u8(cr8, cr8);

        let y0 = vld1_u8(y_row.as_ptr().add(px));
        let y1 = vld1_u8(y_row.as_ptr().add(px + 8));
        let (r0, g0, b0) = ycbcr_to_rgb8_neon(y0, cbz.0, crz.0);
        let (r1, g1, b1) = ycbcr_to_rgb8_neon(y1, cbz.1, crz.1);
        vst1_u8(rv.as_mut_ptr(), r0);
        vst1_u8(rv.as_mut_ptr().add(8), r1);
        vst1_u8(gv.as_mut_ptr(), g0);
        vst1_u8(gv.as_mut_ptr().add(8), g1);
        vst1_u8(bv.as_mut_ptr(), b0);
        vst1_u8(bv.as_mut_ptr().add(8), b1);
        interleave_rgb_planar(&rv, &gv, &bv, &mut out_row[px * 3..(px + 16) * 3]);
        px += 16;
    }
    if px < w {
        ycbcr420_to_rgb_row_scalar(&y_row[px..], &cb_row[px >> 1..], &cr_row[px >> 1..], w - px, &mut out_row[px * 3..]);
    }
}

#[cfg(any(target_arch = "arm", target_arch = "aarch64"))]
#[inline]
#[target_feature(enable = "neon")]
unsafe fn ycbcr420a_to_rgba_row_neon(
    y_row: &[u8],
    cb_row: &[u8],
    cr_row: &[u8],
    a_row: &[u8],
    w: usize,
    out_row: &mut [u8],
) {
    let mut px = 0usize;
    let mut rv = [0u8; 16];
    let mut gv = [0u8; 16];
    let mut bv = [0u8; 16];
    let mut av = [0u8; 16];
    while px + 16 <= w {
        let cb8 = vld1_u8(cb_row.as_ptr().add(px >> 1));
        let cr8 = vld1_u8(cr_row.as_ptr().add(px >> 1));
        let cbz = vzip_u8(cb8, cb8);
        let crz = vzip_u8(cr8, cr8);

        let y0 = vld1_u8(y_row.as_ptr().add(px));
        let y1 = vld1_u8(y_row.as_ptr().add(px + 8));
        let (r0, g0, b0) = ycbcr_to_rgb8_neon(y0, cbz.0, crz.0);
        let (r1, g1, b1) = ycbcr_to_rgb8_neon(y1, cbz.1, crz.1);
        let a0 = vld1_u8(a_row.as_ptr().add(px));
        let a1 = vld1_u8(a_row.as_ptr().add(px + 8));

        vst1_u8(rv.as_mut_ptr(), r0);
        vst1_u8(rv.as_mut_ptr().add(8), r1);
        vst1_u8(gv.as_mut_ptr(), g0);
        vst1_u8(gv.as_mut_ptr().add(8), g1);
        vst1_u8(bv.as_mut_ptr(), b0);
        vst1_u8(bv.as_mut_ptr().add(8), b1);
        vst1_u8(av.as_mut_ptr(), a0);
        vst1_u8(av.as_mut_ptr().add(8), a1);
        interleave_rgba_planar(&rv, &gv, &bv, &av, &mut out_row[px * 4..(px + 16) * 4]);
        px += 16;
    }
    if px < w {
        ycbcr420a_to_rgba_row_scalar(
            &y_row[px..],
            &cb_row[px >> 1..],
            &cr_row[px >> 1..],
            &a_row[px..],
            w - px,
            &mut out_row[px * 4..],
        );
    }
}

#[inline]
fn rgb_to_y_row_scalar(src_row: &[u8], y_row: &mut [u8], w: usize) {
    for px in 0..w {
        let base = px * 3;
        let r = src_row[base] as i32;
        let g = src_row[base + 1] as i32;
        let b = src_row[base + 2] as i32;
        y_row[px] = ((77 * r + 150 * g + 29 * b + 128) >> 8) as u8;
    }
}

#[inline]
fn rgba_to_ya_row_scalar(src_row: &[u8], y_row: &mut [u8], a_row: &mut [u8], w: usize) {
    for px in 0..w {
        let base = px * 4;
        let r = src_row[base] as i32;
        let g = src_row[base + 1] as i32;
        let b = src_row[base + 2] as i32;
        y_row[px] = ((77 * r + 150 * g + 29 * b + 128) >> 8) as u8;
        a_row[px] = src_row[base + 3];
    }
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[target_feature(enable = "sse2")]
unsafe fn rgb_to_y_row_sse2(src_row: &[u8], y_row: &mut [u8], w: usize) {
    let zero = _mm_setzero_si128();
    let c77 = _mm_set1_epi16(77);
    let c150 = _mm_set1_epi16(150);
    let c29 = _mm_set1_epi16(29);
    let c128 = _mm_set1_epi16(128);
    let mut px = 0usize;
    let mut rv = [0u8; 16];
    let mut gv = [0u8; 16];
    let mut bv = [0u8; 16];
    while px + 16 <= w {
        let row = &src_row[px * 3..(px + 16) * 3];
        for i in 0..16 {
            let j = i * 3;
            rv[i] = row[j];
            gv[i] = row[j + 1];
            bv[i] = row[j + 2];
        }
        let r8 = _mm_loadu_si128(rv.as_ptr() as *const __m128i);
        let g8 = _mm_loadu_si128(gv.as_ptr() as *const __m128i);
        let b8 = _mm_loadu_si128(bv.as_ptr() as *const __m128i);
        let r_lo = _mm_unpacklo_epi8(r8, zero);
        let r_hi = _mm_unpackhi_epi8(r8, zero);
        let g_lo = _mm_unpacklo_epi8(g8, zero);
        let g_hi = _mm_unpackhi_epi8(g8, zero);
        let b_lo = _mm_unpacklo_epi8(b8, zero);
        let b_hi = _mm_unpackhi_epi8(b8, zero);

        let y_lo = _mm_srli_epi16(
            _mm_add_epi16(
                _mm_add_epi16(
                    _mm_add_epi16(_mm_mullo_epi16(r_lo, c77), _mm_mullo_epi16(g_lo, c150)),
                    _mm_mullo_epi16(b_lo, c29),
                ),
                c128,
            ),
            8,
        );
        let y_hi = _mm_srli_epi16(
            _mm_add_epi16(
                _mm_add_epi16(
                    _mm_add_epi16(_mm_mullo_epi16(r_hi, c77), _mm_mullo_epi16(g_hi, c150)),
                    _mm_mullo_epi16(b_hi, c29),
                ),
                c128,
            ),
            8,
        );
        let y8 = _mm_packus_epi16(y_lo, y_hi);
        _mm_storeu_si128(y_row.as_mut_ptr().add(px) as *mut __m128i, y8);
        px += 16;
    }
    if px < w {
        rgb_to_y_row_scalar(&src_row[px * 3..], &mut y_row[px..], w - px);
    }
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[target_feature(enable = "sse2")]
unsafe fn rgba_to_ya_row_sse2(src_row: &[u8], y_row: &mut [u8], a_row: &mut [u8], w: usize) {
    let zero = _mm_setzero_si128();
    let c77 = _mm_set1_epi16(77);
    let c150 = _mm_set1_epi16(150);
    let c29 = _mm_set1_epi16(29);
    let c128 = _mm_set1_epi16(128);
    let mut px = 0usize;
    let mut rv = [0u8; 16];
    let mut gv = [0u8; 16];
    let mut bv = [0u8; 16];
    let mut av = [0u8; 16];
    while px + 16 <= w {
        let row = &src_row[px * 4..(px + 16) * 4];
        for i in 0..16 {
            let j = i * 4;
            rv[i] = row[j];
            gv[i] = row[j + 1];
            bv[i] = row[j + 2];
            av[i] = row[j + 3];
        }
        let r8 = _mm_loadu_si128(rv.as_ptr() as *const __m128i);
        let g8 = _mm_loadu_si128(gv.as_ptr() as *const __m128i);
        let b8 = _mm_loadu_si128(bv.as_ptr() as *const __m128i);
        let a8 = _mm_loadu_si128(av.as_ptr() as *const __m128i);
        let r_lo = _mm_unpacklo_epi8(r8, zero);
        let r_hi = _mm_unpackhi_epi8(r8, zero);
        let g_lo = _mm_unpacklo_epi8(g8, zero);
        let g_hi = _mm_unpackhi_epi8(g8, zero);
        let b_lo = _mm_unpacklo_epi8(b8, zero);
        let b_hi = _mm_unpackhi_epi8(b8, zero);
        let y_lo = _mm_srli_epi16(
            _mm_add_epi16(
                _mm_add_epi16(
                    _mm_add_epi16(_mm_mullo_epi16(r_lo, c77), _mm_mullo_epi16(g_lo, c150)),
                    _mm_mullo_epi16(b_lo, c29),
                ),
                c128,
            ),
            8,
        );
        let y_hi = _mm_srli_epi16(
            _mm_add_epi16(
                _mm_add_epi16(
                    _mm_add_epi16(_mm_mullo_epi16(r_hi, c77), _mm_mullo_epi16(g_hi, c150)),
                    _mm_mullo_epi16(b_hi, c29),
                ),
                c128,
            ),
            8,
        );
        let y8 = _mm_packus_epi16(y_lo, y_hi);
        _mm_storeu_si128(y_row.as_mut_ptr().add(px) as *mut __m128i, y8);
        _mm_storeu_si128(a_row.as_mut_ptr().add(px) as *mut __m128i, a8);
        px += 16;
    }
    if px < w {
        rgba_to_ya_row_scalar(&src_row[px * 4..], &mut y_row[px..], &mut a_row[px..], w - px);
    }
}

#[cfg(any(target_arch = "arm", target_arch = "aarch64"))]
#[inline]
#[target_feature(enable = "neon")]
unsafe fn rgb8_to_y8_neon(r8: uint8x8_t, g8: uint8x8_t, b8: uint8x8_t) -> uint8x8_t {
    let mut acc = vmull_u8(r8, vdup_n_u8(77));
    acc = vmlal_u8(acc, g8, vdup_n_u8(150));
    acc = vmlal_u8(acc, b8, vdup_n_u8(29));
    acc = vaddq_u16(acc, vdupq_n_u16(128));
    vqmovn_u16(vshrq_n_u16(acc, 8))
}

#[cfg(any(target_arch = "arm", target_arch = "aarch64"))]
#[inline]
#[target_feature(enable = "neon")]
unsafe fn rgb_to_y_row_neon(src_row: &[u8], y_row: &mut [u8], w: usize) {
    let mut px = 0usize;
    let mut rv = [0u8; 16];
    let mut gv = [0u8; 16];
    let mut bv = [0u8; 16];
    while px + 16 <= w {
        let row = &src_row[px * 3..(px + 16) * 3];
        for i in 0..16 {
            let j = i * 3;
            rv[i] = row[j];
            gv[i] = row[j + 1];
            bv[i] = row[j + 2];
        }
        let r0 = vld1_u8(rv.as_ptr());
        let r1 = vld1_u8(rv.as_ptr().add(8));
        let g0 = vld1_u8(gv.as_ptr());
        let g1 = vld1_u8(gv.as_ptr().add(8));
        let b0 = vld1_u8(bv.as_ptr());
        let b1 = vld1_u8(bv.as_ptr().add(8));
        let y0 = rgb8_to_y8_neon(r0, g0, b0);
        let y1 = rgb8_to_y8_neon(r1, g1, b1);
        vst1_u8(y_row.as_mut_ptr().add(px), y0);
        vst1_u8(y_row.as_mut_ptr().add(px + 8), y1);
        px += 16;
    }
    if px < w {
        rgb_to_y_row_scalar(&src_row[px * 3..], &mut y_row[px..], w - px);
    }
}

#[cfg(any(target_arch = "arm", target_arch = "aarch64"))]
#[inline]
#[target_feature(enable = "neon")]
unsafe fn rgba_to_ya_row_neon(src_row: &[u8], y_row: &mut [u8], a_row: &mut [u8], w: usize) {
    let mut px = 0usize;
    let mut rv = [0u8; 16];
    let mut gv = [0u8; 16];
    let mut bv = [0u8; 16];
    let mut av = [0u8; 16];
    while px + 16 <= w {
        let row = &src_row[px * 4..(px + 16) * 4];
        for i in 0..16 {
            let j = i * 4;
            rv[i] = row[j];
            gv[i] = row[j + 1];
            bv[i] = row[j + 2];
            av[i] = row[j + 3];
        }
        let r0 = vld1_u8(rv.as_ptr());
        let r1 = vld1_u8(rv.as_ptr().add(8));
        let g0 = vld1_u8(gv.as_ptr());
        let g1 = vld1_u8(gv.as_ptr().add(8));
        let b0 = vld1_u8(bv.as_ptr());
        let b1 = vld1_u8(bv.as_ptr().add(8));
        let y0 = rgb8_to_y8_neon(r0, g0, b0);
        let y1 = rgb8_to_y8_neon(r1, g1, b1);
        vst1_u8(y_row.as_mut_ptr().add(px), y0);
        vst1_u8(y_row.as_mut_ptr().add(px + 8), y1);
        vst1q_u8(a_row.as_mut_ptr().add(px), vld1q_u8(av.as_ptr()));
        px += 16;
    }
    if px < w {
        rgba_to_ya_row_scalar(&src_row[px * 4..], &mut y_row[px..], &mut a_row[px..], w - px);
    }
}

/// Convert interleaved RGB (3 bytes/pixel) to separate Y, Cb, Cr planes.
/// Y is full resolution (w×h). Cb and Cr are 4:2:0 subsampled: ((w+1)/2) × ((h+1)/2).
/// Returns (Y, Cb, Cr).
pub fn rgb_to_ycbcr420(image: &[u8], w: usize, h: usize) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
    let npix = w * h;
    let cw = (w + 1) / 2;
    let ch = (h + 1) / 2;

    let mut y_plane  = vec![0u8; npix];
    let mut cb_plane = vec![0u8; cw * ch];
    let mut cr_plane = vec![0u8; cw * ch];

    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    let use_sse2 = is_x86_feature_detected!("sse2");
    #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
    let use_sse2 = false;
    #[cfg(target_arch = "arm")]
    let use_neon = is_arm_feature_detected!("neon");
    #[cfg(target_arch = "aarch64")]
    let use_neon = true;
    #[cfg(not(any(target_arch = "arm", target_arch = "aarch64")))]
    let use_neon = false;

    // Full-res Y in integer fixed-point (BT.601 full-range) with SIMD row kernels.
    if npix >= PARALLEL_ENCODE_PIXELS_THRESHOLD {
        y_plane
            .par_chunks_mut(w)
            .enumerate()
            .for_each(|(py, y_row)| {
                let src_row = &image[(py * w * 3)..((py + 1) * w * 3)];
                if use_sse2 {
                    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
                    unsafe { rgb_to_y_row_sse2(src_row, y_row, w) }
                    #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
                    rgb_to_y_row_scalar(src_row, y_row, w);
                } else if use_neon {
                    #[cfg(any(target_arch = "arm", target_arch = "aarch64"))]
                    unsafe { rgb_to_y_row_neon(src_row, y_row, w) }
                    #[cfg(not(any(target_arch = "arm", target_arch = "aarch64")))]
                    rgb_to_y_row_scalar(src_row, y_row, w);
                } else {
                    rgb_to_y_row_scalar(src_row, y_row, w);
                }
            });
    } else {
        y_plane
            .chunks_mut(w)
            .enumerate()
            .for_each(|(py, y_row)| {
                let src_row = &image[(py * w * 3)..((py + 1) * w * 3)];
                if use_sse2 {
                    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
                    unsafe { rgb_to_y_row_sse2(src_row, y_row, w) }
                    #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
                    rgb_to_y_row_scalar(src_row, y_row, w);
                } else if use_neon {
                    #[cfg(any(target_arch = "arm", target_arch = "aarch64"))]
                    unsafe { rgb_to_y_row_neon(src_row, y_row, w) }
                    #[cfg(not(any(target_arch = "arm", target_arch = "aarch64")))]
                    rgb_to_y_row_scalar(src_row, y_row, w);
                } else {
                    rgb_to_y_row_scalar(src_row, y_row, w);
                }
            });
    }

    // Subsampled Cb/Cr: 2x2 box average.
    if npix >= PARALLEL_ENCODE_PIXELS_THRESHOLD {
        cb_plane
            .par_chunks_mut(cw)
            .zip(cr_plane.par_chunks_mut(cw))
            .enumerate()
            .for_each(|(cy, (cb_row, cr_row))| {
                for cx in 0..cw {
                    let mut sum_cb = 0i32;
                    let mut sum_cr = 0i32;
                    let mut count = 0i32;
                    for dy in 0..2usize {
                        for dx in 0..2usize {
                            let px = cx * 2 + dx;
                            let py = cy * 2 + dy;
                            if px < w && py < h {
                                let base = (py * w + px) * 3;
                                let r = image[base] as i32;
                                let g = image[base + 1] as i32;
                                let b = image[base + 2] as i32;
                                sum_cb += ((-43 * r - 85 * g + 128 * b + 128) >> 8) + 128;
                                sum_cr += ((128 * r - 107 * g - 21 * b + 128) >> 8) + 128;
                                count += 1;
                            }
                        }
                    }
                    cb_row[cx] = clamp_u8((sum_cb + (count >> 1)) / count);
                    cr_row[cx] = clamp_u8((sum_cr + (count >> 1)) / count);
                }
            });
    } else {
        for cy in 0..ch {
            for cx in 0..cw {
                let mut sum_cb = 0i32;
                let mut sum_cr = 0i32;
                let mut count = 0i32;
                for dy in 0..2usize {
                    for dx in 0..2usize {
                        let px = cx * 2 + dx;
                        let py = cy * 2 + dy;
                        if px < w && py < h {
                            let base = (py * w + px) * 3;
                            let r = image[base] as i32;
                            let g = image[base + 1] as i32;
                            let b = image[base + 2] as i32;
                            sum_cb += ((-43 * r - 85 * g + 128 * b + 128) >> 8) + 128;
                            sum_cr += ((128 * r - 107 * g - 21 * b + 128) >> 8) + 128;
                            count += 1;
                        }
                    }
                }
                cb_plane[cy * cw + cx] = clamp_u8((sum_cb + (count >> 1)) / count);
                cr_plane[cy * cw + cx] = clamp_u8((sum_cr + (count >> 1)) / count);
            }
        }
    }

    (y_plane, cb_plane, cr_plane)
}

/// Convert interleaved RGBA (4 bytes/pixel) to Y, Cb, Cr, A planes.
/// Y/Cb/Cr same as rgb_to_ycbcr420. A is full resolution.
pub fn rgba_to_ycbcr420a(image: &[u8], w: usize, h: usize) -> (Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>) {
    let npix = w * h;
    let cw = (w + 1) / 2;
    let ch = (h + 1) / 2;

    let mut y_plane  = vec![0u8; npix];
    let mut cb_plane = vec![0u8; cw * ch];
    let mut cr_plane = vec![0u8; cw * ch];
    let mut a_plane  = vec![0u8; npix];

    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    let use_sse2 = is_x86_feature_detected!("sse2");
    #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
    let use_sse2 = false;
    #[cfg(target_arch = "arm")]
    let use_neon = is_arm_feature_detected!("neon");
    #[cfg(target_arch = "aarch64")]
    let use_neon = true;
    #[cfg(not(any(target_arch = "arm", target_arch = "aarch64")))]
    let use_neon = false;

    if npix >= PARALLEL_ENCODE_PIXELS_THRESHOLD {
        y_plane
            .par_chunks_mut(w)
            .zip(a_plane.par_chunks_mut(w))
            .enumerate()
            .for_each(|(py, (y_row, a_row))| {
                let src_row = &image[(py * w * 4)..((py + 1) * w * 4)];
                if use_sse2 {
                    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
                    unsafe { rgba_to_ya_row_sse2(src_row, y_row, a_row, w) }
                    #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
                    rgba_to_ya_row_scalar(src_row, y_row, a_row, w);
                } else if use_neon {
                    #[cfg(any(target_arch = "arm", target_arch = "aarch64"))]
                    unsafe { rgba_to_ya_row_neon(src_row, y_row, a_row, w) }
                    #[cfg(not(any(target_arch = "arm", target_arch = "aarch64")))]
                    rgba_to_ya_row_scalar(src_row, y_row, a_row, w);
                } else {
                    rgba_to_ya_row_scalar(src_row, y_row, a_row, w);
                }
            });
    } else {
        y_plane
            .chunks_mut(w)
            .zip(a_plane.chunks_mut(w))
            .enumerate()
            .for_each(|(py, (y_row, a_row))| {
                let src_row = &image[(py * w * 4)..((py + 1) * w * 4)];
                if use_sse2 {
                    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
                    unsafe { rgba_to_ya_row_sse2(src_row, y_row, a_row, w) }
                    #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
                    rgba_to_ya_row_scalar(src_row, y_row, a_row, w);
                } else if use_neon {
                    #[cfg(any(target_arch = "arm", target_arch = "aarch64"))]
                    unsafe { rgba_to_ya_row_neon(src_row, y_row, a_row, w) }
                    #[cfg(not(any(target_arch = "arm", target_arch = "aarch64")))]
                    rgba_to_ya_row_scalar(src_row, y_row, a_row, w);
                } else {
                    rgba_to_ya_row_scalar(src_row, y_row, a_row, w);
                }
            });
    }

    if npix >= PARALLEL_ENCODE_PIXELS_THRESHOLD {
        cb_plane
            .par_chunks_mut(cw)
            .zip(cr_plane.par_chunks_mut(cw))
            .enumerate()
            .for_each(|(cy, (cb_row, cr_row))| {
                for cx in 0..cw {
                    let mut sum_cb = 0i32;
                    let mut sum_cr = 0i32;
                    let mut count = 0i32;
                    for dy in 0..2usize {
                        for dx in 0..2usize {
                            let px = cx * 2 + dx;
                            let py = cy * 2 + dy;
                            if px < w && py < h {
                                let base = (py * w + px) * 4;
                                let r = image[base] as i32;
                                let g = image[base + 1] as i32;
                                let b = image[base + 2] as i32;
                                sum_cb += ((-43 * r - 85 * g + 128 * b + 128) >> 8) + 128;
                                sum_cr += ((128 * r - 107 * g - 21 * b + 128) >> 8) + 128;
                                count += 1;
                            }
                        }
                    }
                    cb_row[cx] = clamp_u8((sum_cb + (count >> 1)) / count);
                    cr_row[cx] = clamp_u8((sum_cr + (count >> 1)) / count);
                }
            });
    } else {
        for cy in 0..ch {
            for cx in 0..cw {
                let mut sum_cb = 0i32;
                let mut sum_cr = 0i32;
                let mut count = 0i32;
                for dy in 0..2usize {
                    for dx in 0..2usize {
                        let px = cx * 2 + dx;
                        let py = cy * 2 + dy;
                        if px < w && py < h {
                            let base = (py * w + px) * 4;
                            let r = image[base] as i32;
                            let g = image[base + 1] as i32;
                            let b = image[base + 2] as i32;
                            sum_cb += ((-43 * r - 85 * g + 128 * b + 128) >> 8) + 128;
                            sum_cr += ((128 * r - 107 * g - 21 * b + 128) >> 8) + 128;
                            count += 1;
                        }
                    }
                }
                cb_plane[cy * cw + cx] = clamp_u8((sum_cb + (count >> 1)) / count);
                cr_plane[cy * cw + cx] = clamp_u8((sum_cr + (count >> 1)) / count);
            }
        }
    }

    (y_plane, cb_plane, cr_plane, a_plane)
}

/// Reconstruct interleaved RGB from Y (w×h), Cb and Cr ((cw)×(ch)) planes.
/// Cb/Cr are upsampled with nearest-neighbor (fast, matches JPEG baseline).
pub fn ycbcr420_to_rgb(y: &[u8], cb: &[u8], cr: &[u8], w: usize, _h: usize, out: &mut [u8]) {
    let cw = (w + 1) / 2;
    let h = y.len() / w;
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    let use_avx512 = is_x86_feature_detected!("avx512f");
    #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
    let use_avx512 = false;
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    let use_avx2 = is_x86_feature_detected!("avx2");
    #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
    let use_avx2 = false;
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    let use_sse2 = is_x86_feature_detected!("sse2");
    #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
    let use_sse2 = false;
    #[cfg(target_arch = "arm")]
    let use_neon = is_arm_feature_detected!("neon");
    #[cfg(target_arch = "aarch64")]
    let use_neon = true;
    #[cfg(not(any(target_arch = "arm", target_arch = "aarch64")))]
    let use_neon = false;
    let decode_row = |py: usize, out_row: &mut [u8]| {
        let y_row = py * w;
        let c_row = (py / 2) * cw;
        let y_slice = &y[y_row..y_row + w];
        let cb_slice = &cb[c_row..c_row + cw];
        let cr_slice = &cr[c_row..c_row + cw];
        if use_avx512 || use_avx2 {
            #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
            unsafe {
                ycbcr420_to_rgb_row_avx2(y_slice, cb_slice, cr_slice, w, out_row);
            }
            #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
            ycbcr420_to_rgb_row_scalar(y_slice, cb_slice, cr_slice, w, out_row);
        } else if use_sse2 {
            #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
            unsafe {
                ycbcr420_to_rgb_row_sse2(y_slice, cb_slice, cr_slice, w, out_row);
            }
            #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
            ycbcr420_to_rgb_row_scalar(y_slice, cb_slice, cr_slice, w, out_row);
        } else {
            if use_neon {
                #[cfg(any(target_arch = "arm", target_arch = "aarch64"))]
                unsafe {
                    ycbcr420_to_rgb_row_neon(y_slice, cb_slice, cr_slice, w, out_row);
                }
                #[cfg(not(any(target_arch = "arm", target_arch = "aarch64")))]
                ycbcr420_to_rgb_row_scalar(y_slice, cb_slice, cr_slice, w, out_row);
            } else {
                ycbcr420_to_rgb_row_scalar(y_slice, cb_slice, cr_slice, w, out_row);
            }
        }
    };

    if w * h >= PARALLEL_DECODE_PIXELS_THRESHOLD {
        out.par_chunks_mut(w * 3)
            .enumerate()
            .for_each(|(py, out_row)| decode_row(py, out_row));
    } else {
        out.chunks_mut(w * 3)
            .enumerate()
            .for_each(|(py, out_row)| decode_row(py, out_row));
    }
}

/// Reconstruct interleaved RGBA from Y, Cb, Cr, A planes.
pub fn ycbcr420a_to_rgba(y: &[u8], cb: &[u8], cr: &[u8], a: &[u8], w: usize, _h: usize, out: &mut [u8]) {
    let cw = (w + 1) / 2;
    let h = y.len() / w;
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    let use_avx512 = is_x86_feature_detected!("avx512f");
    #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
    let use_avx512 = false;
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    let use_avx2 = is_x86_feature_detected!("avx2");
    #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
    let use_avx2 = false;
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    let use_sse2 = is_x86_feature_detected!("sse2");
    #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
    let use_sse2 = false;
    #[cfg(target_arch = "arm")]
    let use_neon = is_arm_feature_detected!("neon");
    #[cfg(target_arch = "aarch64")]
    let use_neon = true;
    #[cfg(not(any(target_arch = "arm", target_arch = "aarch64")))]
    let use_neon = false;
    let decode_row = |py: usize, out_row: &mut [u8]| {
        let y_row = py * w;
        let c_row = (py / 2) * cw;
        let y_slice = &y[y_row..y_row + w];
        let cb_slice = &cb[c_row..c_row + cw];
        let cr_slice = &cr[c_row..c_row + cw];
        let a_slice = &a[y_row..y_row + w];
        if use_avx512 || use_avx2 {
            #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
            unsafe {
                ycbcr420a_to_rgba_row_avx2(y_slice, cb_slice, cr_slice, a_slice, w, out_row);
            }
            #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
            ycbcr420a_to_rgba_row_scalar(y_slice, cb_slice, cr_slice, a_slice, w, out_row);
        } else if use_sse2 {
            #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
            unsafe {
                ycbcr420a_to_rgba_row_sse2(y_slice, cb_slice, cr_slice, a_slice, w, out_row);
            }
            #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
            ycbcr420a_to_rgba_row_scalar(y_slice, cb_slice, cr_slice, a_slice, w, out_row);
        } else {
            if use_neon {
                #[cfg(any(target_arch = "arm", target_arch = "aarch64"))]
                unsafe {
                    ycbcr420a_to_rgba_row_neon(y_slice, cb_slice, cr_slice, a_slice, w, out_row);
                }
                #[cfg(not(any(target_arch = "arm", target_arch = "aarch64")))]
                ycbcr420a_to_rgba_row_scalar(y_slice, cb_slice, cr_slice, a_slice, w, out_row);
            } else {
                ycbcr420a_to_rgba_row_scalar(y_slice, cb_slice, cr_slice, a_slice, w, out_row);
            }
        }
    };

    if w * h >= PARALLEL_DECODE_PIXELS_THRESHOLD {
        out.par_chunks_mut(w * 4)
            .enumerate()
            .for_each(|(py, out_row)| decode_row(py, out_row));
    } else {
        out.chunks_mut(w * 4)
            .enumerate()
            .for_each(|(py, out_row)| decode_row(py, out_row));
    }
}
