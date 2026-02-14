use crate::bitstream;
use crate::blockizer::Blockizer;
use crate::dct;
use crate::entropy;
use crate::ffi::quantize_block;

/// .bg header: magic "BG" + version (1=grayscale, 2=RGB) + width u32 LE + height u32 LE (11 bytes).
pub const BG_HEADER_SIZE: usize = 3 + 4 + 4;
const BG_MAGIC_GRAY: &[u8; 3] = b"BG\x01";
const BG_MAGIC_RGB: &[u8; 3] = b"BG\x02";

/// Standard JPEG luminance quantization table (quality ~50).
/// Implicit zigzag order: rows 0..7, columns 0..7.
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
}

/// Encode a single plane (width*height bytes) to buffer (no header).
fn encode_one_plane(
    plane: &[u8],
    width: usize,
    height: usize,
    out_buffer: &mut [u8],
    out_position: &mut i32,
) {
    let table = default_quant_table();
    let blockizer = Blockizer::new(width, height);
    let mut blocks = blockizer.generate_blocks(plane);
    for block in blocks.iter_mut() {
        dct::dct(block);
        quantize(&mut block.data, &table);
        entropy::encode_block_to_buffer(block, out_buffer, out_position);
    }
}

/// Encode a grayscale image (8 bpp) to output buffer.
pub fn encode_grayscale(
    image: &[u8],
    width: usize,
    height: usize,
    out_buffer: &mut [u8],
    out_position: &mut i32,
) {
    write_header_version(out_buffer, out_position, 1, width, height);
    encode_one_plane(image, width, height, out_buffer, out_position);
}

/// Encode an RGB image (24 bpp, R G B order per pixel) to output buffer.
/// image.len() must be width * height * 3.
pub fn encode_rgb(
    image: &[u8],
    width: usize,
    height: usize,
    out_buffer: &mut [u8],
    out_position: &mut i32,
) {
    write_header_version(out_buffer, out_position, 2, width, height);
    let n = width * height;
    let mut plane = vec![0u8; n];
    for c in 0..3 {
        for i in 0..n {
            plane[i] = image[i * 3 + c];
        }
        encode_one_plane(&plane, width, height, out_buffer, out_position);
    }
}

