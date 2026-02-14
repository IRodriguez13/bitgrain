use crate::bitstream;
use crate::blockizer::Blockizer;
use crate::dct;
use crate::entropy;
use crate::ffi::quantize_block;

/// Cabecera .bg: magic "BG\x01" + width u32 LE + height u32 LE (11 bytes).
pub const BG_HEADER_SIZE: usize = 3 + 4 + 4;
const BG_MAGIC: &[u8; 3] = b"BG\x01";

/// Tabla de cuantización luminancia estándar JPEG (calidad ~50).
/// Orden zigzag implícito: filas 0..7, columnas 0..7.
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

/// Escribe la cabecera .bg (magic + width + height).
fn write_header(
    out_buffer: &mut [u8],
    out_position: &mut i32,
    width: usize,
    height: usize,
) {
    if (*out_position as usize) + BG_HEADER_SIZE > out_buffer.len() {
        return;
    }
    for &b in BG_MAGIC {
        bitstream::write_byte(out_buffer, out_position, b);
    }
    for b in (width as u32).to_le_bytes() {
        bitstream::write_byte(out_buffer, out_position, b);
    }
    for b in (height as u32).to_le_bytes() {
        bitstream::write_byte(out_buffer, out_position, b);
    }
}

/// Codifica una imagen en escala de grises (8 bpp) al buffer de salida.
/// Pipeline: cabecera → blockize → DCT → cuantización → RLE (entropy) → buffer.
pub fn encode_grayscale(
    image: &[u8],
    width: usize,
    height: usize,
    out_buffer: &mut [u8],
    out_position: &mut i32,
) {
    write_header(out_buffer, out_position, width, height);
    let table = default_quant_table();
    let blockizer = Blockizer::new(width, height);
    let mut blocks = blockizer.generate_blocks(image);

    for block in blocks.iter_mut() {
        dct::dct(block);
        quantize(&mut block.data, &table);
        entropy::encode_block_to_buffer(block, out_buffer, out_position);
    }
}

