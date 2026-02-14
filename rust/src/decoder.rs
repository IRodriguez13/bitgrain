//! Decoder .bg → image (grayscale or RGB).
//! Pipeline: header → RLE per block → dequantize → IDCT → reassemble image.

use crate::block::Block;
use crate::dct;
use crate::encoder;

const HEADER_SIZE: usize = 3 + 4 + 4;
const EOB_RUN: u8 = 0xFF;

/// Decode a single plane from buffer[pos..] and write to out_plane (w*h bytes).
/// Returns Some(new_pos) or None on error.
fn decode_one_plane(
    buffer: &[u8],
    mut pos: usize,
    w: usize,
    h: usize,
    out_plane: &mut [u8],
) -> Option<usize> {
    let quant_table = encoder::default_quant_table();
    let blocks_wide = (w + 7) / 8;
    let blocks_high = (h + 7) / 8;
    let num_blocks = blocks_wide * blocks_high;

    for block_index in 0..num_blocks {
        let mut block = Block::new();

        if pos + 2 > buffer.len() {
            return None;
        }
        block.data[0] = i16::from_le_bytes([buffer[pos], buffer[pos + 1]]);
        pos += 2;

        let mut ac_index: usize = 1;

        loop {
            if pos + 3 > buffer.len() {
                return None;
            }
            let run = buffer[pos];
            let level = i16::from_le_bytes([buffer[pos + 1], buffer[pos + 2]]);
            pos += 3;

            if run == EOB_RUN && level == 0 {
                break;
            }

            for _ in 0..run {
                if ac_index < 64 {
                    block.data[ac_index] = 0;
                    ac_index += 1;
                }
            }
            if ac_index < 64 {
                block.data[ac_index] = level;
                ac_index += 1;
            }
        }

        for i in 0..64 {
            block.data[i] = block.data[i].saturating_mul(quant_table[i]);
        }

        dct::idct(&mut block);

        let by = (block_index / blocks_wide) * 8;
        let bx = (block_index % blocks_wide) * 8;

        for y in 0..8 {
            for x in 0..8 {
                let px = bx + x;
                let py = by + y;
                if py < h && px < w {
                    let pixel = (block.data[y * 8 + x] + 128).clamp(0, 255);
                    out_plane[py * w + px] = pixel as u8;
                }
            }
        }
    }

    Some(pos)
}

/// Decode a .bg stream into pixels (grayscale or RGB per header).
/// out_channels: 1 = grayscale (out_pixels = w*h), 3 = RGB (out_pixels = w*h*3).
/// Requires out_pixels.len() >= width*height*out_channels.
pub fn decode(
    buffer: &[u8],
    out_pixels: &mut [u8],
    out_width: &mut u32,
    out_height: &mut u32,
    out_channels: &mut u32,
) -> bool {
    if buffer.len() < HEADER_SIZE {
        return false;
    }
    if buffer[0] != b'B' || buffer[1] != b'G' {
        return false;
    }
    let version = buffer[2];
    if version != 1 && version != 2 {
        return false;
    }

    let width = u32::from_le_bytes(buffer[3..7].try_into().unwrap());
    let height = u32::from_le_bytes(buffer[7..11].try_into().unwrap());

    if width == 0 || height == 0 || width > 16384 || height > 16384 {
        return false;
    }

    let w = width as usize;
    let h = height as usize;

    if version == 1 {
        let required = w * h;
        if out_pixels.len() < required {
            return false;
        }
        *out_width = width;
        *out_height = height;
        *out_channels = 1;
        decode_one_plane(buffer, HEADER_SIZE, w, h, out_pixels).is_some()
    } else {
        let required = w * h * 3;
        if out_pixels.len() < required {
            return false;
        }
        *out_width = width;
        *out_height = height;
        *out_channels = 3;

        let mut pos = HEADER_SIZE;
        let mut plane = vec![0u8; w * h];

        for c in 0..3 {
            pos = match decode_one_plane(buffer, pos, w, h, &mut plane) {
                Some(p) => p,
                None => return false,
            };
            for i in 0..(w * h) {
                out_pixels[i * 3 + c] = plane[i];
            }
        }
        true
    }
}

/// Decode a .bg to grayscale (version 1 only).
pub fn decode_grayscale(
    buffer: &[u8],
    out_pixels: &mut [u8],
    out_width: &mut u32,
    out_height: &mut u32,
) -> bool {
    let mut ch = 0u32;
    let ok = decode(buffer, out_pixels, out_width, out_height, &mut ch);
    ok && ch == 1
}
