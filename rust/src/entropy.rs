use crate::block::Block;
use crate::bitstream;

/// EOB (End Of Block): run=0xFF, level=0
const EOB_RUN: u8 = 0xFF;

/// Codifica un bloque en RLE y escribe los (run, level) en el buffer.
/// Formato por par: 1 byte run, 2 bytes level (i16 little-endian).
/// Al final escribe EOB (0xFF, 0x00, 0x00).
pub fn encode_block_to_buffer(
    block: &Block,
    buffer: &mut [u8],
    position: &mut i32,
) {
    // DC (1 valor i16)
    if (*position as usize) + 2 <= buffer.len() {
        let dc = block.data[0];
        let dc_bytes = dc.to_le_bytes();
        bitstream::write_byte(buffer, position, dc_bytes[0]);
        bitstream::write_byte(buffer, position, dc_bytes[1]);
    }

    let pairs = rle_encode(block);

    for (run, level) in pairs {
        if (*position as usize) + 3 <= buffer.len() {
            bitstream::write_byte(buffer, position, run);
            let level_bytes = level.to_le_bytes();
            bitstream::write_byte(buffer, position, level_bytes[0]);
            bitstream::write_byte(buffer, position, level_bytes[1]);
        }
    }
    // EOB
    if (*position as usize) + 3 <= buffer.len() {
        bitstream::write_byte(buffer, position, EOB_RUN);
        bitstream::write_byte(buffer, position, 0);
        bitstream::write_byte(buffer, position, 0);
    }
}

pub fn rle_encode(block: &Block) -> Vec<(u8, i16)> {
    let mut result = Vec::new();
    let mut zero_count = 0u8;

    // empezamos desde 1 (AC), ignorando DC
    for &coef in block.data.iter().skip(1) {
        if coef == 0 {
            zero_count = zero_count.saturating_add(1);
        } else {
            result.push((zero_count, coef));
            zero_count = 0;
        }
    }

    result
}

