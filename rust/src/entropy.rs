use crate::block::Block;
use crate::bitstream;
use crate::zigzag::ZIGZAG;

/// EOB (End Of Block): run=0xFF, level=0
const EOB_RUN: u8 = 0xFF;

/// Encode a block in RLE and write (run, level) pairs to buffer. No per-block allocation.
/// Format per pair: 1 byte run, 2 bytes level (i16 little-endian).
/// Writes EOB (0xFF, 0x00, 0x00) at the end.
/// Run > 255 is emitted as multiple (255, 0) pairs.
pub fn encode_block_to_buffer(
    block: &Block,
    buffer: &mut [u8],
    position: &mut i32,
) {
    let pos = *position as usize;
    if pos + 2 <= buffer.len() {
        let dc = block.data[ZIGZAG[0]];
        let dc_bytes = dc.to_le_bytes();
        bitstream::write_byte(buffer, position, dc_bytes[0]);
        bitstream::write_byte(buffer, position, dc_bytes[1]);
    }

    let mut zero_count: u8 = 0;
    for i in 1..64 {
        let level = block.data[ZIGZAG[i]];
        if level == 0 {
            zero_count = zero_count.saturating_add(1);
            if zero_count == 63 {
                if (*position as usize) + 6 <= buffer.len() {
                    bitstream::write_byte(buffer, position, 62);
                    bitstream::write_byte(buffer, position, 0);
                    bitstream::write_byte(buffer, position, 0);
                    bitstream::write_byte(buffer, position, 0);
                    bitstream::write_byte(buffer, position, 0);
                    bitstream::write_byte(buffer, position, 0);
                }
                zero_count = 0;
            }
        } else {
            if (*position as usize) + 3 <= buffer.len() {
                bitstream::write_byte(buffer, position, zero_count);
                let level_bytes = level.to_le_bytes();
                bitstream::write_byte(buffer, position, level_bytes[0]);
                bitstream::write_byte(buffer, position, level_bytes[1]);
            }
            zero_count = 0;
        }
    }

    if (*position as usize) + 3 <= buffer.len() {
        bitstream::write_byte(buffer, position, EOB_RUN);
        bitstream::write_byte(buffer, position, 0);
        bitstream::write_byte(buffer, position, 0);
    }
}

/// RLE encode for tests or external use. Returns (run, level) pairs (AC only).
#[cfg(test)]
pub fn rle_encode(block: &Block) -> Vec<(u8, i16)> {
    let mut result = Vec::new();
    let mut zero_count = 0u8;
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
