//! Decoder .bg → image.
//! Supports all versions:
//!   v1: grayscale, RLE
//!   v2: RGB planar, RLE
//!   v3: RGBA planar, RLE
//!   v4: YCbCr 4:2:0, Huffman  → RGB output
//!   v5: YCbCr 4:2:0 + A, Huffman → RGBA output
//!   v6: YCbCr 4:2:0, Huffman + chroma AC table → RGB output
//!   v7: YCbCr 4:2:0 + A, Huffman + chroma AC table → RGBA output
//!   v8: YCbCr 4:2:0, perceptual quant + chroma AC → RGB output
//!   v9: YCbCr 4:2:0 + A, perceptual quant + chroma AC → RGBA output
//!  v10: YCbCr 4:2:0, perceptual quant + chroma AC + DC delta → RGB output
//!  v11: YCbCr 4:2:0 + A, perceptual quant + chroma AC + DC delta → RGBA output
//!  v12: YCbCr 4:2:0, stronger perceptual quant + chroma AC + DC delta → RGB output
//!  v13: YCbCr 4:2:0 + A, stronger perceptual quant + chroma AC + DC delta → RGBA output
//!  v14: YCbCr 4:2:0, aggressive perceptual quant + chroma AC + DC delta → RGB output
//!  v15: YCbCr 4:2:0 + A, aggressive perceptual quant + chroma AC + DC delta → RGBA output
//!  v16: YCbCr 4:2:0, very aggressive perceptual quant + chroma AC + DC delta → RGB output
//!  v17: YCbCr 4:2:0 + A, very aggressive perceptual quant + chroma AC + DC delta → RGBA output
//!  v18: YCbCr 4:2:0, ultra perceptual + AC sparsify + chroma AC + DC delta → RGB output
//!  v19: YCbCr 4:2:0 + A, ultra perceptual + AC sparsify + chroma AC + DC delta → RGBA output

use crate::block::Block;
use crate::colorspace;
use crate::dct;
use crate::encoder;
use crate::huffman;
use crate::zigzag::ZIGZAG;
use rayon::prelude::*;

const HEADER_SIZE:     usize = 3 + 4 + 4 + 1;
const HEADER_SIZE_OLD: usize = 3 + 4 + 4;
const EOB_RUN: u8 = 0xFF;

// ---------------------------------------------------------------------------
// RLE decode (v1/v2/v3)
// ---------------------------------------------------------------------------

fn decode_rle_one_block(buffer: &[u8], mut pos: usize) -> Option<(Block, usize)> {
    let mut block = Block::new();
    if pos + 2 > buffer.len() { return None; }
    block.data[ZIGZAG[0]] = i16::from_le_bytes([buffer[pos], buffer[pos + 1]]);
    pos += 2;

    let mut ac_index = 1usize;
    loop {
        if pos + 3 > buffer.len() { return None; }
        let run   = buffer[pos];
        let level = i16::from_le_bytes([buffer[pos + 1], buffer[pos + 2]]);
        pos += 3;
        if run == EOB_RUN && level == 0 { break; }
        for _ in 0..run {
            if ac_index < 64 { block.data[ZIGZAG[ac_index]] = 0; ac_index += 1; }
        }
        if ac_index < 64 { block.data[ZIGZAG[ac_index]] = level; ac_index += 1; }
    }
    Some((block, pos))
}

fn decode_rle_to_blocks(buffer: &[u8], mut pos: usize, n: usize) -> Option<(Vec<Block>, usize)> {
    let mut blocks = Vec::with_capacity(n);
    for _ in 0..n {
        let (block, new_pos) = decode_rle_one_block(buffer, pos)?;
        pos = new_pos;
        blocks.push(block);
    }
    Some((blocks, pos))
}

/// Decode one plane (RLE), dequant+IDCT in parallel, write to interleaved output.
fn decode_plane_rle(
    buffer: &[u8], pos: usize,
    w: usize, h: usize,
    quant: &[i16; 64],
    out: &mut [u8], stride: usize, offset: usize,
) -> Option<usize> {
    let bw = (w + 7) / 8;
    let n  = bw * ((h + 7) / 8);

    let (mut blocks, new_pos) = decode_rle_to_blocks(buffer, pos, n)?;

    blocks.par_iter_mut().for_each(|block| {
        for i in 0..64 { block.data[i] = block.data[i].saturating_mul(quant[i]); }
        dct::idct(block);
    });

    for (idx, block) in blocks.iter().enumerate() {
        let by = (idx / bw) * 8;
        let bx = (idx % bw) * 8;
        for y in 0..8 {
            for x in 0..8 {
                let py = by + y; let px = bx + x;
                if py < h && px < w {
                    let i = (py * w + px) * stride + offset;
                    out[i] = (block.data[y * 8 + x] + 128).clamp(0, 255) as u8;
                }
            }
        }
    }
    Some(new_pos)
}

// ---------------------------------------------------------------------------
// Huffman decode (v4/v5)
// ---------------------------------------------------------------------------

/// Decode one plane using Huffman, dequant+IDCT in parallel, write to flat plane buffer.
fn decode_plane_huffman(
    buffer: &[u8], pos: usize,
    w: usize, h: usize,
    quant: &[i16; 64],
    is_chroma: bool,
    use_chroma_ac: bool,
    use_dc_delta: bool,
    plane: &mut [u8],
) -> Option<usize> {
    let bw = (w + 7) / 8;
    let bh = (h + 7) / 8;
    let n  = bw * bh;

    let (mut blocks, new_pos) =
        huffman::decode_plane_with_profile(buffer, pos, n, is_chroma, use_chroma_ac, use_dc_delta)?;

    // Parallel dequant + IDCT
    blocks.par_iter_mut().for_each(|block| {
        for i in 0..64 { block.data[i] = block.data[i].saturating_mul(quant[i]); }
        dct::idct(block);
    });

    // Write to flat plane
    for (idx, block) in blocks.iter().enumerate() {
        let by = (idx / bw) * 8;
        let bx = (idx % bw) * 8;
        for y in 0..8 {
            for x in 0..8 {
                let py = by + y; let px = bx + x;
                if py < h && px < w {
                    let i = py * w + px;
                    if i < plane.len() {
                        plane[i] = (block.data[y * 8 + x] + 128).clamp(0, 255) as u8;
                    }
                }
            }
        }
    }
    Some(new_pos)
}

// ---------------------------------------------------------------------------
// ICC trailer
// ---------------------------------------------------------------------------

fn parse_icc_trailer(buffer: &[u8], pos: usize) -> Option<(Vec<u8>, usize)> {
    if pos + 8 > buffer.len() { return None; }
    if buffer[pos] != b'B' || buffer[pos+1] != b'G' || buffer[pos+2] != b'x' { return None; }
    let chunk_type = buffer[pos + 3];
    let len = u32::from_le_bytes(buffer[pos+4..pos+8].try_into().unwrap()) as usize;
    let data_pos = pos + 8;
    if chunk_type != 1 || data_pos + len > buffer.len() { return None; }
    Some((buffer[data_pos..data_pos+len].to_vec(), data_pos + len))
}

// ---------------------------------------------------------------------------
// Public decode entry point
// ---------------------------------------------------------------------------

pub fn decode(
    buffer: &[u8],
    out_pixels: &mut [u8],
    out_width:  &mut u32,
    out_height: &mut u32,
    out_channels: &mut u32,
    out_icc: Option<&mut Vec<u8>>,
) -> bool {
    if buffer.len() < HEADER_SIZE_OLD {
        return false;
    }
    if buffer[0] != b'B' || buffer[1] != b'G' {
        return false;
    }

    let version = buffer[2];
    if version == 0 || version > 19 {
        return false;
    }

    let width  = u32::from_le_bytes(buffer[3..7].try_into().unwrap());
    let height = u32::from_le_bytes(buffer[7..11].try_into().unwrap());
    let (header_size, quality) = if buffer.len() >= HEADER_SIZE {
        (HEADER_SIZE, buffer[11])
    } else {
        (HEADER_SIZE_OLD, 50u8)
    };
    let q = if quality == 0 { 50 } else { quality };

    if width == 0 || height == 0 || width > 65536 || height > 65536 { return false; }
    let w = width as usize;
    let h = height as usize;

    // ---- v1: grayscale RLE ----
    if version == 1 {
        if out_pixels.len() < w * h { return false; }
        *out_width = width; *out_height = height; *out_channels = 1;
        let quant = encoder::quant_table_for_quality(q);
        let pos = match decode_plane_rle(buffer, header_size, w, h, &quant, out_pixels, 1, 0) {
            Some(p) => p, None => return false,
        };
        if let Some(v) = out_icc {
            if let Some((icc, _)) = parse_icc_trailer(buffer, pos) { *v = icc; }
        }
        return true;
    }

    // ---- v2: RGB planar RLE ----
    if version == 2 {
        if out_pixels.len() < w * h * 3 { return false; }
        *out_width = width; *out_height = height; *out_channels = 3;
        let quant = encoder::quant_table_for_quality(q);
        let mut pos = header_size;
        for c in 0..3 {
            pos = match decode_plane_rle(buffer, pos, w, h, &quant, out_pixels, 3, c) {
                Some(p) => p, None => return false,
            };
        }
        if let Some(v) = out_icc {
            if let Some((icc, _)) = parse_icc_trailer(buffer, pos) { *v = icc; }
        }
        return true;
    }

    // ---- v3: RGBA planar RLE ----
    if version == 3 {
        if out_pixels.len() < w * h * 4 { return false; }
        *out_width = width; *out_height = height; *out_channels = 4;
        let quant = encoder::quant_table_for_quality(q);
        let mut pos = header_size;
        for c in 0..4 {
            pos = match decode_plane_rle(buffer, pos, w, h, &quant, out_pixels, 4, c) {
                Some(p) => p, None => return false,
            };
        }
        if let Some(v) = out_icc {
            if let Some((icc, _)) = parse_icc_trailer(buffer, pos) { *v = icc; }
        }
        return true;
    }

    // ---- v4: YCbCr 4:2:0 + Huffman → RGB ----
    if version == 4 {
        if out_pixels.len() < w * h * 3 {
            return false;
        }
        *out_width = width; *out_height = height; *out_channels = 3;

        let cw = (w + 1) / 2;
        let ch = (h + 1) / 2;
        let luma_q   = encoder::quant_table_for_quality(q);
        let chroma_q = encoder::chroma_quant_table_for_quality(q);

        let mut y_plane  = vec![0u8; w * h];
        let mut cb_plane = vec![0u8; cw * ch];
        let mut cr_plane = vec![0u8; cw * ch];

        let mut pos = header_size;
        pos = match decode_plane_huffman(buffer, pos, w,  h,  &luma_q,   false, false, false, &mut y_plane)  { Some(p) => p, None => return false };
        pos = match decode_plane_huffman(buffer, pos, cw, ch, &chroma_q, true,  false, false, &mut cb_plane) { Some(p) => p, None => return false };
        pos = match decode_plane_huffman(buffer, pos, cw, ch, &chroma_q, true,  false, false, &mut cr_plane) { Some(p) => p, None => return false };

        colorspace::ycbcr420_to_rgb(&y_plane, &cb_plane, &cr_plane, w, h, out_pixels);

        if let Some(v) = out_icc {
            if let Some((icc, _)) = parse_icc_trailer(buffer, pos) { *v = icc; }
        }
        return true;
    }

    // ---- v5: YCbCr 4:2:0 + A + Huffman → RGBA ----
    if version == 5 {
        if out_pixels.len() < w * h * 4 {
            return false;
        }
        *out_width = width; *out_height = height; *out_channels = 4;

        let cw = (w + 1) / 2;
        let ch = (h + 1) / 2;
        let luma_q   = encoder::quant_table_for_quality(q);
        let chroma_q = encoder::chroma_quant_table_for_quality(q);

        let mut y_plane  = vec![0u8; w * h];
        let mut cb_plane = vec![0u8; cw * ch];
        let mut cr_plane = vec![0u8; cw * ch];
        let mut a_plane  = vec![0u8; w * h];

        let mut pos = header_size;
        pos = match decode_plane_huffman(buffer, pos, w,  h,  &luma_q,   false, false, false, &mut y_plane)  { Some(p) => p, None => return false };
        pos = match decode_plane_huffman(buffer, pos, cw, ch, &chroma_q, true,  false, false, &mut cb_plane) { Some(p) => p, None => return false };
        pos = match decode_plane_huffman(buffer, pos, cw, ch, &chroma_q, true,  false, false, &mut cr_plane) { Some(p) => p, None => return false };
        pos = match decode_plane_huffman(buffer, pos, w,  h,  &luma_q,   false, false, false, &mut a_plane)  { Some(p) => p, None => return false };

        colorspace::ycbcr420a_to_rgba(&y_plane, &cb_plane, &cr_plane, &a_plane, w, h, out_pixels);

        if let Some(v) = out_icc {
            if let Some((icc, _)) = parse_icc_trailer(buffer, pos) { *v = icc; }
        }
        return true;
    }

    // ---- v6: YCbCr 4:2:0 + Huffman (chroma AC) → RGB ----
    if version == 6 {
        if out_pixels.len() < w * h * 3 {
            return false;
        }
        *out_width = width; *out_height = height; *out_channels = 3;

        let cw = (w + 1) / 2;
        let ch = (h + 1) / 2;
        let luma_q   = encoder::quant_table_for_quality(q);
        let chroma_q = encoder::chroma_quant_table_for_quality(q);

        let mut y_plane  = vec![0u8; w * h];
        let mut cb_plane = vec![0u8; cw * ch];
        let mut cr_plane = vec![0u8; cw * ch];

        let mut pos = header_size;
        pos = match decode_plane_huffman(buffer, pos, w,  h,  &luma_q,   false, false, false, &mut y_plane)  { Some(p) => p, None => return false };
        pos = match decode_plane_huffman(buffer, pos, cw, ch, &chroma_q, true,  true,  false, &mut cb_plane) { Some(p) => p, None => return false };
        pos = match decode_plane_huffman(buffer, pos, cw, ch, &chroma_q, true,  true,  false, &mut cr_plane) { Some(p) => p, None => return false };

        colorspace::ycbcr420_to_rgb(&y_plane, &cb_plane, &cr_plane, w, h, out_pixels);

        if let Some(v) = out_icc {
            if let Some((icc, _)) = parse_icc_trailer(buffer, pos) { *v = icc; }
        }
        return true;
    }

    // ---- v7: YCbCr 4:2:0 + A + Huffman (chroma AC) → RGBA ----
    if version == 7 {
        if out_pixels.len() < w * h * 4 {
            return false;
        }
        *out_width = width; *out_height = height; *out_channels = 4;

        let cw = (w + 1) / 2;
        let ch = (h + 1) / 2;
        let luma_q   = encoder::quant_table_for_quality(q);
        let chroma_q = encoder::chroma_quant_table_for_quality(q);

        let mut y_plane  = vec![0u8; w * h];
        let mut cb_plane = vec![0u8; cw * ch];
        let mut cr_plane = vec![0u8; cw * ch];
        let mut a_plane  = vec![0u8; w * h];

        let mut pos = header_size;
        pos = match decode_plane_huffman(buffer, pos, w,  h,  &luma_q,   false, false, false, &mut y_plane)  { Some(p) => p, None => return false };
        pos = match decode_plane_huffman(buffer, pos, cw, ch, &chroma_q, true,  true,  false, &mut cb_plane) { Some(p) => p, None => return false };
        pos = match decode_plane_huffman(buffer, pos, cw, ch, &chroma_q, true,  true,  false, &mut cr_plane) { Some(p) => p, None => return false };
        pos = match decode_plane_huffman(buffer, pos, w,  h,  &luma_q,   false, false, false, &mut a_plane)  { Some(p) => p, None => return false };

        colorspace::ycbcr420a_to_rgba(&y_plane, &cb_plane, &cr_plane, &a_plane, w, h, out_pixels);

        if let Some(v) = out_icc {
            if let Some((icc, _)) = parse_icc_trailer(buffer, pos) { *v = icc; }
        }
        return true;
    }

    // ---- v8: YCbCr 4:2:0 + Huffman (perceptual quant + chroma AC) → RGB ----
    if version == 8 {
        if out_pixels.len() < w * h * 3 {
            return false;
        }
        *out_width = width; *out_height = height; *out_channels = 3;

        let cw = (w + 1) / 2;
        let ch = (h + 1) / 2;
        let luma_q   = encoder::quant_table_for_quality_perceptual(q);
        let chroma_q = encoder::chroma_quant_table_for_quality_perceptual(q);

        let mut y_plane  = vec![0u8; w * h];
        let mut cb_plane = vec![0u8; cw * ch];
        let mut cr_plane = vec![0u8; cw * ch];

        let mut pos = header_size;
        pos = match decode_plane_huffman(buffer, pos, w,  h,  &luma_q,   false, false, false, &mut y_plane)  { Some(p) => p, None => return false };
        pos = match decode_plane_huffman(buffer, pos, cw, ch, &chroma_q, true,  true,  false, &mut cb_plane) { Some(p) => p, None => return false };
        pos = match decode_plane_huffman(buffer, pos, cw, ch, &chroma_q, true,  true,  false, &mut cr_plane) { Some(p) => p, None => return false };

        colorspace::ycbcr420_to_rgb(&y_plane, &cb_plane, &cr_plane, w, h, out_pixels);

        if let Some(v) = out_icc {
            if let Some((icc, _)) = parse_icc_trailer(buffer, pos) { *v = icc; }
        }
        return true;
    }

    // ---- v9: YCbCr 4:2:0 + A + Huffman (perceptual quant + chroma AC) → RGBA ----
    if version == 9 {
        if out_pixels.len() < w * h * 4 {
            return false;
        }
        *out_width = width; *out_height = height; *out_channels = 4;

        let cw = (w + 1) / 2;
        let ch = (h + 1) / 2;
        let luma_q   = encoder::quant_table_for_quality_perceptual(q);
        let chroma_q = encoder::chroma_quant_table_for_quality_perceptual(q);

        let mut y_plane  = vec![0u8; w * h];
        let mut cb_plane = vec![0u8; cw * ch];
        let mut cr_plane = vec![0u8; cw * ch];
        let mut a_plane  = vec![0u8; w * h];

        let mut pos = header_size;
        pos = match decode_plane_huffman(buffer, pos, w,  h,  &luma_q,   false, false, false, &mut y_plane)  { Some(p) => p, None => return false };
        pos = match decode_plane_huffman(buffer, pos, cw, ch, &chroma_q, true,  true,  false, &mut cb_plane) { Some(p) => p, None => return false };
        pos = match decode_plane_huffman(buffer, pos, cw, ch, &chroma_q, true,  true,  false, &mut cr_plane) { Some(p) => p, None => return false };
        pos = match decode_plane_huffman(buffer, pos, w,  h,  &luma_q,   false, false, false, &mut a_plane)  { Some(p) => p, None => return false };

        colorspace::ycbcr420a_to_rgba(&y_plane, &cb_plane, &cr_plane, &a_plane, w, h, out_pixels);

        if let Some(v) = out_icc {
            if let Some((icc, _)) = parse_icc_trailer(buffer, pos) { *v = icc; }
        }
        return true;
    }

    // ---- v10: YCbCr 4:2:0 + Huffman (perceptual quant + chroma AC + DC delta) → RGB ----
    if version == 10 {
        if out_pixels.len() < w * h * 3 {
            return false;
        }
        *out_width = width; *out_height = height; *out_channels = 3;

        let cw = (w + 1) / 2;
        let ch = (h + 1) / 2;
        let luma_q   = encoder::quant_table_for_quality_perceptual(q);
        let chroma_q = encoder::chroma_quant_table_for_quality_perceptual(q);

        let mut y_plane  = vec![0u8; w * h];
        let mut cb_plane = vec![0u8; cw * ch];
        let mut cr_plane = vec![0u8; cw * ch];

        let mut pos = header_size;
        pos = match decode_plane_huffman(buffer, pos, w,  h,  &luma_q,   false, false, true, &mut y_plane)  { Some(p) => p, None => return false };
        pos = match decode_plane_huffman(buffer, pos, cw, ch, &chroma_q, true,  true,  true, &mut cb_plane) { Some(p) => p, None => return false };
        pos = match decode_plane_huffman(buffer, pos, cw, ch, &chroma_q, true,  true,  true, &mut cr_plane) { Some(p) => p, None => return false };

        colorspace::ycbcr420_to_rgb(&y_plane, &cb_plane, &cr_plane, w, h, out_pixels);

        if let Some(v) = out_icc {
            if let Some((icc, _)) = parse_icc_trailer(buffer, pos) { *v = icc; }
        }
        return true;
    }

    // ---- v11: YCbCr 4:2:0 + A + Huffman (perceptual quant + chroma AC + DC delta) → RGBA ----
    if version == 11 {
        if out_pixels.len() < w * h * 4 {
            return false;
        }
        *out_width = width; *out_height = height; *out_channels = 4;

        let cw = (w + 1) / 2;
        let ch = (h + 1) / 2;
        let luma_q   = encoder::quant_table_for_quality_perceptual(q);
        let chroma_q = encoder::chroma_quant_table_for_quality_perceptual(q);

        let mut y_plane  = vec![0u8; w * h];
        let mut cb_plane = vec![0u8; cw * ch];
        let mut cr_plane = vec![0u8; cw * ch];
        let mut a_plane  = vec![0u8; w * h];

        let mut pos = header_size;
        pos = match decode_plane_huffman(buffer, pos, w,  h,  &luma_q,   false, false, true, &mut y_plane)  { Some(p) => p, None => return false };
        pos = match decode_plane_huffman(buffer, pos, cw, ch, &chroma_q, true,  true,  true, &mut cb_plane) { Some(p) => p, None => return false };
        pos = match decode_plane_huffman(buffer, pos, cw, ch, &chroma_q, true,  true,  true, &mut cr_plane) { Some(p) => p, None => return false };
        pos = match decode_plane_huffman(buffer, pos, w,  h,  &luma_q,   false, false, true, &mut a_plane)  { Some(p) => p, None => return false };

        colorspace::ycbcr420a_to_rgba(&y_plane, &cb_plane, &cr_plane, &a_plane, w, h, out_pixels);

        if let Some(v) = out_icc {
            if let Some((icc, _)) = parse_icc_trailer(buffer, pos) { *v = icc; }
        }
        return true;
    }

    // ---- v12: YCbCr 4:2:0 + Huffman (strong perceptual quant + chroma AC + DC delta) → RGB ----
    if version == 12 {
        if out_pixels.len() < w * h * 3 {
            return false;
        }
        *out_width = width; *out_height = height; *out_channels = 3;

        let cw = (w + 1) / 2;
        let ch = (h + 1) / 2;
        let luma_q   = encoder::quant_table_for_quality_perceptual_v2(q);
        let chroma_q = encoder::chroma_quant_table_for_quality_perceptual_v2(q);

        let mut y_plane  = vec![0u8; w * h];
        let mut cb_plane = vec![0u8; cw * ch];
        let mut cr_plane = vec![0u8; cw * ch];

        let mut pos = header_size;
        pos = match decode_plane_huffman(buffer, pos, w,  h,  &luma_q,   false, false, true, &mut y_plane)  { Some(p) => p, None => return false };
        pos = match decode_plane_huffman(buffer, pos, cw, ch, &chroma_q, true,  true,  true, &mut cb_plane) { Some(p) => p, None => return false };
        pos = match decode_plane_huffman(buffer, pos, cw, ch, &chroma_q, true,  true,  true, &mut cr_plane) { Some(p) => p, None => return false };

        colorspace::ycbcr420_to_rgb(&y_plane, &cb_plane, &cr_plane, w, h, out_pixels);

        if let Some(v) = out_icc {
            if let Some((icc, _)) = parse_icc_trailer(buffer, pos) { *v = icc; }
        }
        return true;
    }

    // ---- v13: YCbCr 4:2:0 + A + Huffman (strong perceptual quant + chroma AC + DC delta) → RGBA ----
    if version == 13 {
        if out_pixels.len() < w * h * 4 {
            return false;
        }
        *out_width = width; *out_height = height; *out_channels = 4;

        let cw = (w + 1) / 2;
        let ch = (h + 1) / 2;
        let luma_q   = encoder::quant_table_for_quality_perceptual_v2(q);
        let chroma_q = encoder::chroma_quant_table_for_quality_perceptual_v2(q);

        let mut y_plane  = vec![0u8; w * h];
        let mut cb_plane = vec![0u8; cw * ch];
        let mut cr_plane = vec![0u8; cw * ch];
        let mut a_plane  = vec![0u8; w * h];

        let mut pos = header_size;
        pos = match decode_plane_huffman(buffer, pos, w,  h,  &luma_q,   false, false, true, &mut y_plane)  { Some(p) => p, None => return false };
        pos = match decode_plane_huffman(buffer, pos, cw, ch, &chroma_q, true,  true,  true, &mut cb_plane) { Some(p) => p, None => return false };
        pos = match decode_plane_huffman(buffer, pos, cw, ch, &chroma_q, true,  true,  true, &mut cr_plane) { Some(p) => p, None => return false };
        pos = match decode_plane_huffman(buffer, pos, w,  h,  &luma_q,   false, false, true, &mut a_plane)  { Some(p) => p, None => return false };

        colorspace::ycbcr420a_to_rgba(&y_plane, &cb_plane, &cr_plane, &a_plane, w, h, out_pixels);

        if let Some(v) = out_icc {
            if let Some((icc, _)) = parse_icc_trailer(buffer, pos) { *v = icc; }
        }
        return true;
    }

    // ---- v14: YCbCr 4:2:0 + Huffman (aggressive perceptual quant + chroma AC + DC delta) → RGB ----
    if version == 14 {
        if out_pixels.len() < w * h * 3 {
            return false;
        }
        *out_width = width; *out_height = height; *out_channels = 3;

        let cw = (w + 1) / 2;
        let ch = (h + 1) / 2;
        let luma_q   = encoder::quant_table_for_quality_perceptual_v3(q);
        let chroma_q = encoder::chroma_quant_table_for_quality_perceptual_v3(q);

        let mut y_plane  = vec![0u8; w * h];
        let mut cb_plane = vec![0u8; cw * ch];
        let mut cr_plane = vec![0u8; cw * ch];

        let mut pos = header_size;
        pos = match decode_plane_huffman(buffer, pos, w,  h,  &luma_q,   false, false, true, &mut y_plane)  { Some(p) => p, None => return false };
        pos = match decode_plane_huffman(buffer, pos, cw, ch, &chroma_q, true,  true,  true, &mut cb_plane) { Some(p) => p, None => return false };
        pos = match decode_plane_huffman(buffer, pos, cw, ch, &chroma_q, true,  true,  true, &mut cr_plane) { Some(p) => p, None => return false };

        colorspace::ycbcr420_to_rgb(&y_plane, &cb_plane, &cr_plane, w, h, out_pixels);

        if let Some(v) = out_icc {
            if let Some((icc, _)) = parse_icc_trailer(buffer, pos) { *v = icc; }
        }
        return true;
    }

    // ---- v15: YCbCr 4:2:0 + A + Huffman (aggressive perceptual quant + chroma AC + DC delta) → RGBA ----
    if version == 15 {
        if out_pixels.len() < w * h * 4 {
            return false;
        }
        *out_width = width; *out_height = height; *out_channels = 4;

        let cw = (w + 1) / 2;
        let ch = (h + 1) / 2;
        let luma_q   = encoder::quant_table_for_quality_perceptual_v3(q);
        let chroma_q = encoder::chroma_quant_table_for_quality_perceptual_v3(q);

        let mut y_plane  = vec![0u8; w * h];
        let mut cb_plane = vec![0u8; cw * ch];
        let mut cr_plane = vec![0u8; cw * ch];
        let mut a_plane  = vec![0u8; w * h];

        let mut pos = header_size;
        pos = match decode_plane_huffman(buffer, pos, w,  h,  &luma_q,   false, false, true, &mut y_plane)  { Some(p) => p, None => return false };
        pos = match decode_plane_huffman(buffer, pos, cw, ch, &chroma_q, true,  true,  true, &mut cb_plane) { Some(p) => p, None => return false };
        pos = match decode_plane_huffman(buffer, pos, cw, ch, &chroma_q, true,  true,  true, &mut cr_plane) { Some(p) => p, None => return false };
        pos = match decode_plane_huffman(buffer, pos, w,  h,  &luma_q,   false, false, true, &mut a_plane)  { Some(p) => p, None => return false };

        colorspace::ycbcr420a_to_rgba(&y_plane, &cb_plane, &cr_plane, &a_plane, w, h, out_pixels);

        if let Some(v) = out_icc {
            if let Some((icc, _)) = parse_icc_trailer(buffer, pos) { *v = icc; }
        }
        return true;
    }

    // ---- v16: YCbCr 4:2:0 + Huffman (very aggressive perceptual quant + chroma AC + DC delta) → RGB ----
    if version == 16 {
        if out_pixels.len() < w * h * 3 {
            return false;
        }
        *out_width = width; *out_height = height; *out_channels = 3;

        let cw = (w + 1) / 2;
        let ch = (h + 1) / 2;
        let luma_q   = encoder::quant_table_for_quality_perceptual_v4(q);
        let chroma_q = encoder::chroma_quant_table_for_quality_perceptual_v4(q);

        let mut y_plane  = vec![0u8; w * h];
        let mut cb_plane = vec![0u8; cw * ch];
        let mut cr_plane = vec![0u8; cw * ch];

        let mut pos = header_size;
        pos = match decode_plane_huffman(buffer, pos, w,  h,  &luma_q,   false, false, true, &mut y_plane)  { Some(p) => p, None => return false };
        pos = match decode_plane_huffman(buffer, pos, cw, ch, &chroma_q, true,  true,  true, &mut cb_plane) { Some(p) => p, None => return false };
        pos = match decode_plane_huffman(buffer, pos, cw, ch, &chroma_q, true,  true,  true, &mut cr_plane) { Some(p) => p, None => return false };

        colorspace::ycbcr420_to_rgb(&y_plane, &cb_plane, &cr_plane, w, h, out_pixels);

        if let Some(v) = out_icc {
            if let Some((icc, _)) = parse_icc_trailer(buffer, pos) { *v = icc; }
        }
        return true;
    }

    // ---- v17: YCbCr 4:2:0 + A + Huffman (very aggressive perceptual quant + chroma AC + DC delta) → RGBA ----
    if version == 17 {
        if out_pixels.len() < w * h * 4 {
            return false;
        }
        *out_width = width; *out_height = height; *out_channels = 4;

        let cw = (w + 1) / 2;
        let ch = (h + 1) / 2;
        let luma_q   = encoder::quant_table_for_quality_perceptual_v4(q);
        let chroma_q = encoder::chroma_quant_table_for_quality_perceptual_v4(q);

        let mut y_plane  = vec![0u8; w * h];
        let mut cb_plane = vec![0u8; cw * ch];
        let mut cr_plane = vec![0u8; cw * ch];
        let mut a_plane  = vec![0u8; w * h];

        let mut pos = header_size;
        pos = match decode_plane_huffman(buffer, pos, w,  h,  &luma_q,   false, false, true, &mut y_plane)  { Some(p) => p, None => return false };
        pos = match decode_plane_huffman(buffer, pos, cw, ch, &chroma_q, true,  true,  true, &mut cb_plane) { Some(p) => p, None => return false };
        pos = match decode_plane_huffman(buffer, pos, cw, ch, &chroma_q, true,  true,  true, &mut cr_plane) { Some(p) => p, None => return false };
        pos = match decode_plane_huffman(buffer, pos, w,  h,  &luma_q,   false, false, true, &mut a_plane)  { Some(p) => p, None => return false };

        colorspace::ycbcr420a_to_rgba(&y_plane, &cb_plane, &cr_plane, &a_plane, w, h, out_pixels);

        if let Some(v) = out_icc {
            if let Some((icc, _)) = parse_icc_trailer(buffer, pos) { *v = icc; }
        }
        return true;
    }

    // ---- v18: YCbCr 4:2:0 + Huffman (ultra perceptual + AC sparsify + chroma AC + DC delta) → RGB ----
    if version == 18 {
        if out_pixels.len() < w * h * 3 {
            return false;
        }
        *out_width = width; *out_height = height; *out_channels = 3;

        let cw = (w + 1) / 2;
        let ch = (h + 1) / 2;
        let luma_q   = encoder::quant_table_for_quality_perceptual_v4(q);
        let chroma_q = encoder::chroma_quant_table_for_quality_perceptual_v4(q);

        let mut y_plane  = vec![0u8; w * h];
        let mut cb_plane = vec![0u8; cw * ch];
        let mut cr_plane = vec![0u8; cw * ch];

        let mut pos = header_size;
        pos = match decode_plane_huffman(buffer, pos, w,  h,  &luma_q,   false, false, true, &mut y_plane)  { Some(p) => p, None => return false };
        pos = match decode_plane_huffman(buffer, pos, cw, ch, &chroma_q, true,  true,  true, &mut cb_plane) { Some(p) => p, None => return false };
        pos = match decode_plane_huffman(buffer, pos, cw, ch, &chroma_q, true,  true,  true, &mut cr_plane) { Some(p) => p, None => return false };

        colorspace::ycbcr420_to_rgb(&y_plane, &cb_plane, &cr_plane, w, h, out_pixels);

        if let Some(v) = out_icc {
            if let Some((icc, _)) = parse_icc_trailer(buffer, pos) { *v = icc; }
        }
        return true;
    }

    // ---- v19: YCbCr 4:2:0 + A + Huffman (ultra perceptual + AC sparsify + chroma AC + DC delta) → RGBA ----
    if version == 19 {
        if out_pixels.len() < w * h * 4 {
            return false;
        }
        *out_width = width; *out_height = height; *out_channels = 4;

        let cw = (w + 1) / 2;
        let ch = (h + 1) / 2;
        let luma_q   = encoder::quant_table_for_quality_perceptual_v4(q);
        let chroma_q = encoder::chroma_quant_table_for_quality_perceptual_v4(q);

        let mut y_plane  = vec![0u8; w * h];
        let mut cb_plane = vec![0u8; cw * ch];
        let mut cr_plane = vec![0u8; cw * ch];
        let mut a_plane  = vec![0u8; w * h];

        let mut pos = header_size;
        pos = match decode_plane_huffman(buffer, pos, w,  h,  &luma_q,   false, false, true, &mut y_plane)  { Some(p) => p, None => return false };
        pos = match decode_plane_huffman(buffer, pos, cw, ch, &chroma_q, true,  true,  true, &mut cb_plane) { Some(p) => p, None => return false };
        pos = match decode_plane_huffman(buffer, pos, cw, ch, &chroma_q, true,  true,  true, &mut cr_plane) { Some(p) => p, None => return false };
        pos = match decode_plane_huffman(buffer, pos, w,  h,  &luma_q,   false, false, true, &mut a_plane)  { Some(p) => p, None => return false };

        colorspace::ycbcr420a_to_rgba(&y_plane, &cb_plane, &cr_plane, &a_plane, w, h, out_pixels);

        if let Some(v) = out_icc {
            if let Some((icc, _)) = parse_icc_trailer(buffer, pos) { *v = icc; }
        }
        return true;
    }

    false
}

pub fn decode_grayscale(
    buffer: &[u8], out_pixels: &mut [u8],
    out_width: &mut u32, out_height: &mut u32,
) -> bool {
    let mut ch = 0u32;
    let ok = decode(buffer, out_pixels, out_width, out_height, &mut ch, None);
    ok && ch == 1
}
