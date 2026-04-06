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
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
use std::arch::is_x86_feature_detected;
#[cfg(target_arch = "x86")]
use std::arch::x86::*;
#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;
#[inline]
fn clamp_u8(v: i32) -> u8 {
    if v < 0 { 0 } else if v > 255 { 255 } else { v as u8 }
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
    while px + 16 <= w {
        let mut r_add_arr = [0i16; 16];
        let mut g_sub_arr = [0i16; 16];
        let mut b_add_arr = [0i16; 16];
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

        let mut rv = [0u8; 16];
        let mut gv = [0u8; 16];
        let mut bv = [0u8; 16];
        _mm_storeu_si128(rv.as_mut_ptr() as *mut __m128i, r8);
        _mm_storeu_si128(gv.as_mut_ptr() as *mut __m128i, g8);
        _mm_storeu_si128(bv.as_mut_ptr() as *mut __m128i, b8);

        for i in 0..16usize {
            let o = (px + i) * 3;
            out_row[o] = rv[i];
            out_row[o + 1] = gv[i];
            out_row[o + 2] = bv[i];
        }
        px += 16;
    }

    if px < w {
        ycbcr420_to_rgb_row_scalar(&y_row[px..], &cb_row[px >> 1..], &cr_row[px >> 1..], w - px, &mut out_row[px * 3..]);
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

    // Full-res Y in integer fixed-point (BT.601 full-range).
    for i in 0..npix {
        let r = image[i * 3] as i32;
        let g = image[i * 3 + 1] as i32;
        let b = image[i * 3 + 2] as i32;
        y_plane[i] = ((77 * r + 150 * g + 29 * b + 128) >> 8) as u8;
    }

    // Subsampled Cb/Cr: 2×2 box average
    for cy in 0..ch {
        for cx in 0..cw {
            let mut sum_cb = 0i32;
            let mut sum_cr = 0i32;
            let mut count  = 0i32;
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
                        count  += 1;
                    }
                }
            }
            cb_plane[cy * cw + cx] = clamp_u8((sum_cb + (count >> 1)) / count);
            cr_plane[cy * cw + cx] = clamp_u8((sum_cr + (count >> 1)) / count);
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

    for i in 0..npix {
        let r = image[i * 4] as i32;
        let g = image[i * 4 + 1] as i32;
        let b = image[i * 4 + 2] as i32;
        y_plane[i] = ((77 * r + 150 * g + 29 * b + 128) >> 8) as u8;
        a_plane[i] = image[i * 4 + 3];
    }

    for cy in 0..ch {
        for cx in 0..cw {
            let mut sum_cb = 0i32;
            let mut sum_cr = 0i32;
            let mut count  = 0i32;
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
                        count  += 1;
                    }
                }
            }
            cb_plane[cy * cw + cx] = clamp_u8((sum_cb + (count >> 1)) / count);
            cr_plane[cy * cw + cx] = clamp_u8((sum_cr + (count >> 1)) / count);
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
    let use_sse2 = is_x86_feature_detected!("sse2");
    #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
    let use_sse2 = false;
    let decode_row = |py: usize, out_row: &mut [u8]| {
        let y_row = py * w;
        let c_row = (py / 2) * cw;
        let y_slice = &y[y_row..y_row + w];
        let cb_slice = &cb[c_row..c_row + cw];
        let cr_slice = &cr[c_row..c_row + cw];
        if use_sse2 {
            #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
            unsafe {
                ycbcr420_to_rgb_row_sse2(y_slice, cb_slice, cr_slice, w, out_row);
            }
            #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
            ycbcr420_to_rgb_row_scalar(y_slice, cb_slice, cr_slice, w, out_row);
        } else {
            ycbcr420_to_rgb_row_scalar(y_slice, cb_slice, cr_slice, w, out_row);
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
    let decode_row = |py: usize, out_row: &mut [u8]| {
        let y_row = py * w;
        let c_row = (py / 2) * cw;
        let mut px = 0usize;
        while px + 1 < w {
            let cbi = cb[c_row + (px >> 1)] as i32 - 128;
            let cri = cr[c_row + (px >> 1)] as i32 - 128;
            let r_add = (359 * cri) >> 8;
            let g_sub = (88 * cbi + 183 * cri) >> 8;
            let b_add = (454 * cbi) >> 8;

            let yi0 = y[y_row + px] as i32;
            let o0 = px * 4;
            out_row[o0]     = clamp_u8(yi0 + r_add);
            out_row[o0 + 1] = clamp_u8(yi0 - g_sub);
            out_row[o0 + 2] = clamp_u8(yi0 + b_add);
            out_row[o0 + 3] = a[y_row + px];

            let yi1 = y[y_row + px + 1] as i32;
            let o1 = o0 + 4;
            out_row[o1]     = clamp_u8(yi1 + r_add);
            out_row[o1 + 1] = clamp_u8(yi1 - g_sub);
            out_row[o1 + 2] = clamp_u8(yi1 + b_add);
            out_row[o1 + 3] = a[y_row + px + 1];

            px += 2;
        }
        if px < w {
            let cbi = cb[c_row + (px >> 1)] as i32 - 128;
            let cri = cr[c_row + (px >> 1)] as i32 - 128;
            let yi = y[y_row + px] as i32;
            let i = px * 4;
            out_row[i]     = clamp_u8(yi + ((359 * cri) >> 8));
            out_row[i + 1] = clamp_u8(yi - ((88 * cbi + 183 * cri) >> 8));
            out_row[i + 2] = clamp_u8(yi + ((454 * cbi) >> 8));
            out_row[i + 3] = a[y_row + px];
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
