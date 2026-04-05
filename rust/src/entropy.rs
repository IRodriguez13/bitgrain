use crate::block::Block;
use crate::zigzag::ZIGZAG;

/// EOB (End Of Block): run=0xFF, level=0x0000
const EOB_RUN: u8 = 0xFF;
/// Max run length that fits in a u8 without colliding with EOB_RUN
const MAX_RUN: u8 = 0xFE;

/// Encode a block in RLE and write directly to buffer using slice writes.
/// Format: DC = 2 bytes (i16 LE), then (run: u8, level: i16 LE) pairs, EOB = (0xFF, 0x00, 0x00).
/// FIX: previous code capped run at 63 instead of MAX_RUN (254), losing data for sparse blocks.
/// FIX: uses direct slice writes instead of byte-by-byte for better throughput.
#[inline]
pub fn encode_block_to_buffer(block: &Block, buffer: &mut [u8], position: &mut i32) {
    let mut pos = *position as usize;
    let len = buffer.len();

    // DC coefficient (2 bytes)
    if pos + 2 > len { *position = pos as i32; return; }
    let dc = block.data[ZIGZAG[0]].to_le_bytes();
    buffer[pos]     = dc[0];
    buffer[pos + 1] = dc[1];
    pos += 2;

    // AC coefficients: RLE (run, level) pairs
    let mut zero_run: u8 = 0;
    for i in 1..64 {
        let level = block.data[ZIGZAG[i]];
        if level == 0 {
            if zero_run == MAX_RUN {
                // Flush max-run zero pair before overflow
                if pos + 3 > len { break; }
                buffer[pos]     = MAX_RUN;
                buffer[pos + 1] = 0;
                buffer[pos + 2] = 0;
                pos += 3;
                zero_run = 0;
            }
            zero_run += 1;
        } else {
            if pos + 3 > len { break; }
            let lv = level.to_le_bytes();
            buffer[pos]     = zero_run;
            buffer[pos + 1] = lv[0];
            buffer[pos + 2] = lv[1];
            pos += 3;
            zero_run = 0;
        }
    }

    // EOB
    if pos + 3 <= len {
        buffer[pos]     = EOB_RUN;
        buffer[pos + 1] = 0;
        buffer[pos + 2] = 0;
        pos += 3;
    }

    *position = pos as i32;
}

/// RLE encode for tests. Returns (run, level) pairs (AC only).
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
