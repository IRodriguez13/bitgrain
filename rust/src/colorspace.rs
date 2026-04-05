//! RGB ↔ YCbCr color space conversion and 4:2:0 chroma subsampling.
//!
//! Uses BT.601 full-range coefficients (same as JPEG):
//!   Y  =  0.299·R + 0.587·G + 0.114·B
//!   Cb = -0.168736·R - 0.331264·G + 0.5·B + 128
//!   Cr =  0.5·R - 0.418688·G - 0.081312·B + 128
//!
//! 4:2:0 subsampling: Cb and Cr planes are downsampled to (ceil(W/2)) × (ceil(H/2))
//! using a simple 2×2 box average, matching JPEG's default chroma subsampling.

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

    // Full-res Y
    for i in 0..npix {
        let r = image[i * 3]     as f32;
        let g = image[i * 3 + 1] as f32;
        let b = image[i * 3 + 2] as f32;
        y_plane[i] = (0.299 * r + 0.587 * g + 0.114 * b).round().clamp(0.0, 255.0) as u8;
    }

    // Subsampled Cb/Cr: 2×2 box average
    for cy in 0..ch {
        for cx in 0..cw {
            let mut sum_cb = 0f32;
            let mut sum_cr = 0f32;
            let mut count  = 0f32;
            for dy in 0..2usize {
                for dx in 0..2usize {
                    let px = cx * 2 + dx;
                    let py = cy * 2 + dy;
                    if px < w && py < h {
                        let r = image[(py * w + px) * 3]     as f32;
                        let g = image[(py * w + px) * 3 + 1] as f32;
                        let b = image[(py * w + px) * 3 + 2] as f32;
                        sum_cb += -0.168736 * r - 0.331264 * g + 0.5     * b + 128.0;
                        sum_cr +=  0.5      * r - 0.418688 * g - 0.081312 * b + 128.0;
                        count  += 1.0;
                    }
                }
            }
            cb_plane[cy * cw + cx] = (sum_cb / count).round().clamp(0.0, 255.0) as u8;
            cr_plane[cy * cw + cx] = (sum_cr / count).round().clamp(0.0, 255.0) as u8;
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
        let r = image[i * 4]     as f32;
        let g = image[i * 4 + 1] as f32;
        let b = image[i * 4 + 2] as f32;
        y_plane[i] = (0.299 * r + 0.587 * g + 0.114 * b).round().clamp(0.0, 255.0) as u8;
        a_plane[i] = image[i * 4 + 3];
    }

    for cy in 0..ch {
        for cx in 0..cw {
            let mut sum_cb = 0f32;
            let mut sum_cr = 0f32;
            let mut count  = 0f32;
            for dy in 0..2usize {
                for dx in 0..2usize {
                    let px = cx * 2 + dx;
                    let py = cy * 2 + dy;
                    if px < w && py < h {
                        let r = image[(py * w + px) * 4]     as f32;
                        let g = image[(py * w + px) * 4 + 1] as f32;
                        let b = image[(py * w + px) * 4 + 2] as f32;
                        sum_cb += -0.168736 * r - 0.331264 * g + 0.5      * b + 128.0;
                        sum_cr +=  0.5      * r - 0.418688 * g - 0.081312 * b + 128.0;
                        count  += 1.0;
                    }
                }
            }
            cb_plane[cy * cw + cx] = (sum_cb / count).round().clamp(0.0, 255.0) as u8;
            cr_plane[cy * cw + cx] = (sum_cr / count).round().clamp(0.0, 255.0) as u8;
        }
    }

    (y_plane, cb_plane, cr_plane, a_plane)
}

/// Reconstruct interleaved RGB from Y (w×h), Cb and Cr ((cw)×(ch)) planes.
/// Cb/Cr are upsampled with nearest-neighbor (fast, matches JPEG baseline).
pub fn ycbcr420_to_rgb(y: &[u8], cb: &[u8], cr: &[u8], w: usize, h: usize, out: &mut [u8]) {
    #[inline]
    fn clamp_u8(v: i32) -> u8 {
        if v < 0 { 0 } else if v > 255 { 255 } else { v as u8 }
    }

    let cw = (w + 1) / 2;
    for py in 0..h {
        let y_row = py * w;
        let c_row = (py / 2) * cw;
        let out_row = py * w * 3;
        for px in 0..w {
            let yi = y[y_row + px] as i32;
            let cbi = cb[c_row + (px / 2)] as i32 - 128;
            let cri = cr[c_row + (px / 2)] as i32 - 128;

            // Fixed-point BT.601 full-range (close to float path, faster in hot decode loop).
            let r = yi + ((359 * cri) >> 8);
            let g = yi - ((88 * cbi + 183 * cri) >> 8);
            let b = yi + ((454 * cbi) >> 8);

            let i = out_row + px * 3;
            out[i]     = clamp_u8(r);
            out[i + 1] = clamp_u8(g);
            out[i + 2] = clamp_u8(b);
        }
    }
}

/// Reconstruct interleaved RGBA from Y, Cb, Cr, A planes.
pub fn ycbcr420a_to_rgba(y: &[u8], cb: &[u8], cr: &[u8], a: &[u8], w: usize, h: usize, out: &mut [u8]) {
    #[inline]
    fn clamp_u8(v: i32) -> u8 {
        if v < 0 { 0 } else if v > 255 { 255 } else { v as u8 }
    }

    let cw = (w + 1) / 2;
    for py in 0..h {
        let y_row = py * w;
        let c_row = (py / 2) * cw;
        let out_row = py * w * 4;
        for px in 0..w {
            let yi = y[y_row + px] as i32;
            let cbi = cb[c_row + (px / 2)] as i32 - 128;
            let cri = cr[c_row + (px / 2)] as i32 - 128;

            let r = yi + ((359 * cri) >> 8);
            let g = yi - ((88 * cbi + 183 * cri) >> 8);
            let b = yi + ((454 * cbi) >> 8);

            let i = out_row + px * 4;
            out[i]     = clamp_u8(r);
            out[i + 1] = clamp_u8(g);
            out[i + 2] = clamp_u8(b);
            out[i + 3] = a[y_row + px];
        }
    }
}
