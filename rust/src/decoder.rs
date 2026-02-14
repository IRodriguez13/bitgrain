//! Decodificador .bg → imagen en escala de grises.
//! Pipeline: cabecera → RLE por bloque → dequantizar → IDCT → rearmar imagen.

use crate::block::Block;
use crate::dct;
use crate::encoder;

const BG_MAGIC: &[u8; 3] = b"BG\x01";
const HEADER_SIZE: usize = 3 + 4 + 4; // magic + width + height
const EOB_RUN: u8 = 0xFF;

/// Decodifica un flujo .bg (con cabecera) en píxeles.
/// Requiere que out_pixels.len() >= width * height.
/// Devuelve true si ok, false si datos inválidos o buffer insuficiente.
pub fn decode_grayscale(
    buffer: &[u8],
    out_pixels: &mut [u8],
    out_width: &mut u32,
    out_height: &mut u32,
) -> bool {
    if buffer.len() < HEADER_SIZE {
        return false;
    }
    if buffer[0..3] != BG_MAGIC[..] {
        return false;
    }

    let width = u32::from_le_bytes(buffer[3..7].try_into().unwrap());
    let height = u32::from_le_bytes(buffer[7..11].try_into().unwrap());

    if width == 0 || height == 0 || width > 16384 || height > 16384 {
        return false;
    }
    if (width % 8) != 0 || (height % 8) != 0 {
        return false;
    }

    let w = width as usize;
    let h = height as usize;
    let required = w * h;
    if out_pixels.len() < required {
        return false;
    }

    *out_width = width;
    *out_height = height;

    let quant_table = encoder::default_quant_table();
    let blocks_wide = w / 8;
    let blocks_high = h / 8;
    let num_blocks = blocks_wide * blocks_high;

    let mut pos = HEADER_SIZE;

    for block_index in 0..num_blocks {
        let mut block = Block::new();

        if pos + 2 > buffer.len() {
            return false;
        }
        block.data[0] = i16::from_le_bytes([buffer[pos], buffer[pos + 1]]);
        pos += 2;

        let mut ac_index: usize = 1;

        loop {
            if pos + 3 > buffer.len() {
                return false;
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
                let pixel = (block.data[y * 8 + x] + 128).clamp(0, 255);
                let idx = (by + y) * w + (bx + x);
                out_pixels[idx] = pixel as u8;
            }
        }
    }

    true
}
