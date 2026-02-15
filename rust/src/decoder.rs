//! Decoder .bg → image (grayscale or RGB).
//! Pipeline: header → RLE per block (sequential) → dequant + IDCT + write (parallel).

use crate::block::Block;
use crate::dct;
use crate::encoder;
use crate::zigzag::ZIGZAG;

const HEADER_SIZE: usize = 3 + 4 + 4 + 1;
const HEADER_SIZE_OLD: usize = 3 + 4 + 4;
const EOB_RUN: u8 = 0xFF;

/// Decode a single block from buffer[pos..]. Returns Some((block, new_pos)) or None.
fn decode_rle_one_block(buffer: &[u8], mut pos: usize) -> Option<(Block, usize)> {
    let mut block = Block::new();
    if pos + 2 > buffer.len() {
        return None;
    }
    block.data[ZIGZAG[0]] = i16::from_le_bytes([buffer[pos], buffer[pos + 1]]);
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
                block.data[ZIGZAG[ac_index]] = 0;
                ac_index += 1;
            }
        }
        if ac_index < 64 {
            block.data[ZIGZAG[ac_index]] = level;
            ac_index += 1;
        }
    }
    Some((block, pos))
}

/// Decode RLE only: buffer[pos..] → Vec<Block>. Returns Some((blocks, new_pos)) or None.
fn decode_rle_to_blocks(buffer: &[u8], mut pos: usize, num_blocks: usize) -> Option<(Vec<Block>, usize)> {
    let mut blocks = Vec::with_capacity(num_blocks);

    for _ in 0..num_blocks {
        let (block, new_pos) = decode_rle_one_block(buffer, pos)?;
        pos = new_pos;
        blocks.push(block);
    }

    Some((blocks, pos))
}

/// Decode block-by-block (streaming): no full temp, write directly to out.
fn decode_blocks_streaming(
    buffer: &[u8],
    mut pos: usize,
    w: usize,
    h: usize,
    quant_table: &[i16; 64],
    out: &mut [u8],
    stride: usize,
    offset: usize,
) -> Option<usize> {
    let blocks_wide = (w + 7) / 8;
    let blocks_high = (h + 7) / 8;
    let num_blocks = blocks_wide * blocks_high;

    for block_index in 0..num_blocks {
        let (mut block, new_pos) = decode_rle_one_block(buffer, pos)?;
        pos = new_pos;

        for i in 0..64 {
            block.data[i] = block.data[i].saturating_mul(quant_table[i]);
        }
        dct::idct(&mut block);

        let by = (block_index / blocks_wide) * 8;
        let bx = (block_index % blocks_wide) * 8;
        for y in 0..8 {
            for x in 0..8 {
                let py = by + y;
                let px = bx + x;
                if py < h && px < w {
                    let idx = (py * w + px) * stride + offset;
                    if idx < out.len() {
                        out[idx] = (block.data[y * 8 + x] + 128).clamp(0, 255) as u8;
                    }
                }
            }
        }
    }
    Some(pos)
}

/// Decode a single plane from buffer[pos..] and write to out. Streaming: block-by-block, no full temp.
fn decode_one_plane_strided(
    buffer: &[u8],
    pos: usize,
    w: usize,
    h: usize,
    quant_table: &[i16; 64],
    out: &mut [u8],
    stride: usize,
    offset: usize,
) -> Option<usize> {
    decode_blocks_streaming(buffer, pos, w, h, quant_table, out, stride, offset)
}

/// Parse optional ICC trailer at buffer[pos..]. Returns (icc_data, new_pos) or None.
fn parse_icc_trailer(buffer: &[u8], pos: usize) -> Option<(Vec<u8>, usize)> {
    if pos + 3 + 1 + 4 > buffer.len() {
        return None;
    }
    if buffer[pos] != b'B' || buffer[pos + 1] != b'G' || buffer[pos + 2] != b'x' {
        return None;
    }
    let chunk_type = buffer[pos + 3];
    let len = u32::from_le_bytes(buffer[pos + 4..pos + 8].try_into().unwrap()) as usize;
    let pos = pos + 8;
    if chunk_type != 1 || pos + len > buffer.len() {
        return None;
    }
    Some((buffer[pos..pos + len].to_vec(), pos + len))
}

/// Decode a .bg stream into pixels (grayscale or RGB per header).
/// out_channels: 1 = grayscale (out_pixels = w*h), 3 = RGB (out_pixels = w*h*3).
/// Requires out_pixels.len() >= width*height*out_channels.
/// out_icc: if Some, receives embedded ICC profile (caller must free).
pub fn decode(
    buffer: &[u8],
    out_pixels: &mut [u8],
    out_width: &mut u32,
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
    if version != 1 && version != 2 && version != 3 {
        return false;
    }

    let width = u32::from_le_bytes(buffer[3..7].try_into().unwrap());
    let height = u32::from_le_bytes(buffer[7..11].try_into().unwrap());
    let (header_size, quality) = if buffer.len() >= HEADER_SIZE {
        (HEADER_SIZE, buffer[11])
    } else {
        (HEADER_SIZE_OLD, 50u8)
    };
    let quant_table = encoder::quant_table_for_quality(if quality == 0 { 50 } else { quality });

    if width == 0 || height == 0 || width > 65536 || height > 65536 {
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
        let pos = match decode_one_plane_strided(buffer, header_size, w, h, &quant_table, out_pixels, 1, 0) {
            Some(p) => p,
            None => return false,
        };
        if let Some(v) = out_icc {
            if let Some((icc, _)) = parse_icc_trailer(buffer, pos) {
                *v = icc;
            }
        }
        true
    } else {
        let ch = if version == 3 { 4 } else { 3 };
        let required = w * h * ch;
        if out_pixels.len() < required {
            return false;
        }
        *out_width = width;
        *out_height = height;
        *out_channels = ch as u32;

        let mut pos = header_size;
        for c in 0..ch {
            pos = match decode_one_plane_strided(buffer, pos, w, h, &quant_table, out_pixels, ch, c) {
                Some(p) => p,
                None => return false,
            };
        }
        if let Some(v) = out_icc {
            if let Some((icc, _)) = parse_icc_trailer(buffer, pos) {
                *v = icc;
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
    let ok = decode(buffer, out_pixels, out_width, out_height, &mut ch, None);
    ok && ch == 1
}
