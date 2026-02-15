use crate::bitstream;
use crate::block::Block;
use crate::blockizer::Blockizer;
use crate::dct;
use crate::entropy;
use crate::ffi::quantize_block;
use rayon::prelude::*;

/// .bg header: magic "BG" + version (1=gray, 2=RGB, 3=RGBA) + width u32 LE + height u32 LE + quality u8 (12 bytes).
/// Quality 0 in file means default 50 (backward compat with 11-byte header).
pub const BG_HEADER_SIZE: usize = 3 + 4 + 4 + 1;
const BG_MAGIC_GRAY: &[u8; 3] = b"BG\x01";
const BG_MAGIC_RGB: &[u8; 3] = b"BG\x02";
const BG_MAGIC_RGBA: &[u8; 3] = b"BG\x03";

/// Standard JPEG luminance quantization table (quality ~50).
pub fn default_quant_table() -> [i16; 64] {
    [
        16, 11, 10, 16, 24, 40, 51, 61,
        12, 12, 14, 19, 26, 58, 60, 55,
        14, 13, 16, 24, 40, 57, 69, 56,
        14, 17, 22, 29, 51, 87, 80, 62,
        18, 22, 37, 56, 68, 109, 103, 77,
        24, 35, 55, 64, 81, 104, 113, 92,
        49, 64, 78, 87, 103, 121, 120, 101,
        72, 92, 95, 98, 112, 100, 103, 99,
    ]
}

/// Scale quant table by quality (1–100). Higher quality = less quantization.
/// quality 50 ≈ default table; 100 = minimal loss; 1 = heavy compression.
pub fn quant_table_for_quality(quality: u8) -> [i16; 64] {
    let q = quality.clamp(1, 100) as i32;
    let base = default_quant_table();
    let mut out = [0i16; 64];
    for i in 0..64 {
        let v = (base[i] as i32 * q + 50) / 100;
        out[i] = v.clamp(1, 255) as i16;
    }
    out
}

pub fn quantize(block: &mut [i16; 64], table: &[i16; 64]) {
    unsafe {
        quantize_block(block.as_mut_ptr(), table.as_ptr());
    }
}

/// Write .bg header (magic with version + width + height).
fn write_header_version(
    out_buffer: &mut [u8],
    out_position: &mut i32,
    version: u8,
    width: usize,
    height: usize,
    quality: u8,
) {
    if (*out_position as usize) + BG_HEADER_SIZE > out_buffer.len() {
        return;
    }
    bitstream::write_byte(out_buffer, out_position, b'B');
    bitstream::write_byte(out_buffer, out_position, b'G');
    bitstream::write_byte(out_buffer, out_position, version);
    for b in (width as u32).to_le_bytes() {
        bitstream::write_byte(out_buffer, out_position, b);
    }
    for b in (height as u32).to_le_bytes() {
        bitstream::write_byte(out_buffer, out_position, b);
    }
    bitstream::write_byte(out_buffer, out_position, if quality == 0 { 50 } else { quality });
}

/// Encode a sequence of blocks (DCT, quantize in parallel, then RLE to buffer in order).
fn encode_blocks(
    blocks: &mut [Block],
    table: &[i16; 64],
    out_buffer: &mut [u8],
    out_position: &mut i32,
) {
    blocks.par_iter_mut().for_each(|block| {
        dct::dct(block);
        quantize(&mut block.data, table);
    });
    for block in blocks.iter() {
        entropy::encode_block_to_buffer(block, out_buffer, out_position);
    }
}

/// Encode a single plane (width*height bytes) to buffer (no header).
fn encode_one_plane(
    plane: &[u8],
    width: usize,
    height: usize,
    quality: u8,
    out_buffer: &mut [u8],
    out_position: &mut i32,
) {
    let table = quant_table_for_quality(quality);
    let blockizer = Blockizer::new(width, height);
    let mut blocks = blockizer.generate_blocks(plane);
    encode_blocks(&mut blocks, &table, out_buffer, out_position);
}

/// Encode a grayscale image (8 bpp) to output buffer.
/// quality: 1–100 (higher = less quantization, default 85).
pub fn encode_grayscale(
    image: &[u8],
    width: usize,
    height: usize,
    quality: u8,
    out_buffer: &mut [u8],
    out_position: &mut i32,
) {
    write_header_version(out_buffer, out_position, 1, width, height, quality);
    encode_one_plane(image, width, height, quality, out_buffer, out_position);
}

/// Write optional ICC trailer: "BGx" + chunk type 1 + length LE + data.
fn write_icc_trailer(
    out_buffer: &mut [u8],
    out_position: &mut i32,
    icc: Option<&[u8]>,
) {
    let Some(icc_data) = icc else { return };
    if icc_data.is_empty() {
        return;
    }
    let pos = *out_position as usize;
    let need = 3 + 1 + 4 + icc_data.len(); /* "BGx" + type + len + data */
    if pos + need > out_buffer.len() {
        return;
    }
    bitstream::write_byte(out_buffer, out_position, b'B');
    bitstream::write_byte(out_buffer, out_position, b'G');
    bitstream::write_byte(out_buffer, out_position, b'x');
    bitstream::write_byte(out_buffer, out_position, 1); /* chunk type: ICC */
    for b in (icc_data.len() as u32).to_le_bytes() {
        bitstream::write_byte(out_buffer, out_position, b);
    }
    bitstream::write_bytes(out_buffer, out_position, icc_data);
}

/// Encode an RGB image (24 bpp, R G B order per pixel) to output buffer.
/// image.len() must be width * height * 3. No intermediate plane allocation.
/// icc: optional ICC profile to embed (for color management).
pub fn encode_rgb(
    image: &[u8],
    width: usize,
    height: usize,
    quality: u8,
    out_buffer: &mut [u8],
    out_position: &mut i32,
    icc: Option<&[u8]>,
) {
    write_header_version(out_buffer, out_position, 2, width, height, quality);
    let table = quant_table_for_quality(quality);
    let blockizer = Blockizer::new(width, height);
    for c in 0..3 {
        let mut blocks = blockizer.generate_blocks_rgb(image, c);
        encode_blocks(&mut blocks, &table, out_buffer, out_position);
    }
    write_icc_trailer(out_buffer, out_position, icc);
}

/// Encode an RGBA image (32 bpp, R G B A order per pixel) to output buffer.
/// image.len() must be width * height * 4.
/// icc: optional ICC profile to embed (for color management).
pub fn encode_rgba(
    image: &[u8],
    width: usize,
    height: usize,
    quality: u8,
    out_buffer: &mut [u8],
    out_position: &mut i32,
    icc: Option<&[u8]>,
) {
    write_header_version(out_buffer, out_position, 3, width, height, quality);
    let table = quant_table_for_quality(quality);
    let blockizer = Blockizer::new(width, height);
    for c in 0..4 {
        let mut blocks = blockizer.generate_blocks_rgba(image, c);
        encode_blocks(&mut blocks, &table, out_buffer, out_position);
    }
    write_icc_trailer(out_buffer, out_position, icc);
}

