//! RGB ↔ YCbCr color space conversion and 4:2:0 chroma subsampling.
//!
//! Uses BT.601 full-range coefficients (same as JPEG):
//!   Y  =  0.299·R + 0.587·G + 0.114·B
//!   Cb = -0.168736·R - 0.331264·G + 0.5·B + 128
//!   Cr =  0.5·R - 0.418688·G - 0.081312·B + 128
//!
//! 4:2:0 subsampling: Cb and Cr planes are downsampled to (ceil(W/2)) × (ceil(H/2))
//! using a simple 2×2 box average, matching JPEG's default chroma subsampling.
#[inline]
fn clamp_u8(v: i32) -> u8 {
    if v < 0 { 0 } else if v > 255 { 255 } else { v as u8 }
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
pub fn ycbcr420_to_rgb(y: &[u8], cb: &[u8], cr: &[u8], w: usize, h: usize, out: &mut [u8]) {
    let cw = (w + 1) / 2;
    for py in 0..h {
        let y_row = py * w;
        let c_row = (py / 2) * cw;
        let out_row = py * w * 3;
        let mut px = 0usize;
        while px + 1 < w {
            let cbi = cb[c_row + (px >> 1)] as i32 - 128;
            let cri = cr[c_row + (px >> 1)] as i32 - 128;
            let r_add = (359 * cri) >> 8;
            let g_sub = (88 * cbi + 183 * cri) >> 8;
            let b_add = (454 * cbi) >> 8;

            let yi0 = y[y_row + px] as i32;
            let o0 = out_row + px * 3;
            out[o0]     = clamp_u8(yi0 + r_add);
            out[o0 + 1] = clamp_u8(yi0 - g_sub);
            out[o0 + 2] = clamp_u8(yi0 + b_add);

            let yi1 = y[y_row + px + 1] as i32;
            let o1 = o0 + 3;
            out[o1]     = clamp_u8(yi1 + r_add);
            out[o1 + 1] = clamp_u8(yi1 - g_sub);
            out[o1 + 2] = clamp_u8(yi1 + b_add);

            px += 2;
        }
        if px < w {
            let cbi = cb[c_row + (px >> 1)] as i32 - 128;
            let cri = cr[c_row + (px >> 1)] as i32 - 128;
            let yi = y[y_row + px] as i32;
            let i = out_row + px * 3;
            out[i]     = clamp_u8(yi + ((359 * cri) >> 8));
            out[i + 1] = clamp_u8(yi - ((88 * cbi + 183 * cri) >> 8));
            out[i + 2] = clamp_u8(yi + ((454 * cbi) >> 8));
        }
    }
}

/// Reconstruct interleaved RGBA from Y, Cb, Cr, A planes.
pub fn ycbcr420a_to_rgba(y: &[u8], cb: &[u8], cr: &[u8], a: &[u8], w: usize, h: usize, out: &mut [u8]) {
    let cw = (w + 1) / 2;
    for py in 0..h {
        let y_row = py * w;
        let c_row = (py / 2) * cw;
        let out_row = py * w * 4;
        let mut px = 0usize;
        while px + 1 < w {
            let cbi = cb[c_row + (px >> 1)] as i32 - 128;
            let cri = cr[c_row + (px >> 1)] as i32 - 128;
            let r_add = (359 * cri) >> 8;
            let g_sub = (88 * cbi + 183 * cri) >> 8;
            let b_add = (454 * cbi) >> 8;

            let yi0 = y[y_row + px] as i32;
            let o0 = out_row + px * 4;
            out[o0]     = clamp_u8(yi0 + r_add);
            out[o0 + 1] = clamp_u8(yi0 - g_sub);
            out[o0 + 2] = clamp_u8(yi0 + b_add);
            out[o0 + 3] = a[y_row + px];

            let yi1 = y[y_row + px + 1] as i32;
            let o1 = o0 + 4;
            out[o1]     = clamp_u8(yi1 + r_add);
            out[o1 + 1] = clamp_u8(yi1 - g_sub);
            out[o1 + 2] = clamp_u8(yi1 + b_add);
            out[o1 + 3] = a[y_row + px + 1];

            px += 2;
        }
        if px < w {
            let cbi = cb[c_row + (px >> 1)] as i32 - 128;
            let cri = cr[c_row + (px >> 1)] as i32 - 128;
            let yi = y[y_row + px] as i32;
            let i = out_row + px * 4;
            out[i]     = clamp_u8(yi + ((359 * cri) >> 8));
            out[i + 1] = clamp_u8(yi - ((88 * cbi + 183 * cri) >> 8));
            out[i + 2] = clamp_u8(yi + ((454 * cbi) >> 8));
            out[i + 3] = a[y_row + px];
        }
    }
}
