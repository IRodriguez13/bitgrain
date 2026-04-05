use crate::block::Block;
use crate::huffman::{clamp_block_jpeg_coeffs, decode_plane, encode_plane};
use crate::zigzag::ZIGZAG;

fn make_block(vals: &[(usize, i16)]) -> Block {
    let mut b = Block::new();
    for &(zi, v) in vals {
        b.data[ZIGZAG[zi]] = v;
    }
    b
}

#[test]
fn huffman_roundtrip_single_block() {
    let block = make_block(&[(0, 100), (1, 50), (2, -30), (5, 10)]);
    let encoded = encode_plane(&[block], false);
    let (decoded, _) = decode_plane(&encoded, 0, 1, false).expect("decode failed");
    assert_eq!(decoded[0].data, block.data, "roundtrip mismatch");
}

#[test]
fn huffman_roundtrip_all_zeros() {
    let block = Block::new();
    let encoded = encode_plane(&[block], false);
    let (decoded, _) = decode_plane(&encoded, 0, 1, false).expect("decode failed");
    assert_eq!(decoded[0].data, block.data);
}

#[test]
fn huffman_roundtrip_multiple_blocks() {
    let blocks: Vec<Block> = (0..16i16)
        .map(|i| make_block(&[(0, i * 10), (1, i * 3 - 20), (3, i - 5)]))
        .collect();
    let encoded = encode_plane(&blocks, false);
    let (decoded, _) = decode_plane(&encoded, 0, 16, false).expect("decode failed");
    for (i, (orig, dec)) in blocks.iter().zip(decoded.iter()).enumerate() {
        assert_eq!(orig.data, dec.data, "block {i} mismatch");
    }
}

#[test]
fn huffman_roundtrip_chroma() {
    let block = make_block(&[(0, 20), (1, -15), (4, 8)]);
    let encoded = encode_plane(&[block], true);
    let (decoded, _) = decode_plane(&encoded, 0, 1, true).expect("chroma decode failed");
    assert_eq!(decoded[0].data, block.data);
}

#[test]
fn huffman_long_zero_run_rs_symbol() {
    let mut b = Block::new();
    b.data[ZIGZAG[16]] = 1;
    let encoded = encode_plane(&[b], false);
    let (dec, _) = decode_plane(&encoded, 0, 1, false).expect("decode");
    assert_eq!(dec[0].data, b.data);
}

#[test]
fn huffman_roundtrip_after_jpeg_clamp() {
    let mut b = Block::new();
    b.data[ZIGZAG[1]] = 1500;
    clamp_block_jpeg_coeffs(&mut b);
    assert_eq!(b.data[ZIGZAG[1]], 1023);
    let encoded = encode_plane(&[b], false);
    let (dec, _) = decode_plane(&encoded, 0, 1, false).expect("decode");
    assert_eq!(dec[0].data, b.data);
}

#[test]
fn huffman_consecutive_planes() {
    let y_blocks: Vec<Block> = (0..4).map(|i| make_block(&[(0, i * 20), (1, i * 5)])).collect();
    let cb_blocks: Vec<Block> = (0..1).map(|i| make_block(&[(0, i * 10 + 5)])).collect();

    let y_buf = encode_plane(&y_blocks, false);
    let cb_buf = encode_plane(&cb_blocks, true);

    let mut combined = y_buf.clone();
    combined.extend_from_slice(&cb_buf);

    let (y_dec, y_end) = decode_plane(&combined, 0, 4, false).expect("Y decode");
    let (cb_dec, _) = decode_plane(&combined, y_end, 1, true).expect("Cb decode");

    for (i, (o, d)) in y_blocks.iter().zip(y_dec.iter()).enumerate() {
        assert_eq!(o.data, d.data, "Y block {i}");
    }
    for (i, (o, d)) in cb_blocks.iter().zip(cb_dec.iter()).enumerate() {
        assert_eq!(o.data, d.data, "Cb block {i}");
    }
}

#[test]
fn huffman_roundtrip_stress_random_blocks() {
    // Deterministic LCG to avoid external rand dependency.
    fn next_u32(state: &mut u64) -> u32 {
        *state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        (*state >> 32) as u32
    }

    let mut seed = 0x1234_5678_9abc_def0u64;
    let n_blocks = 4096usize;
    let mut blocks = Vec::with_capacity(n_blocks);
    for _ in 0..n_blocks {
        let mut b = Block::new();
        // Dense, wide-range random coefficients to stress symbol coverage.
        b.data[ZIGZAG[0]] = (next_u32(&mut seed) % 4095) as i16 - 2047;
        for zi in 1..64 {
            let r = next_u32(&mut seed) % 100;
            if r < 65 {
                b.data[ZIGZAG[zi]] = 0;
            } else {
                b.data[ZIGZAG[zi]] = (next_u32(&mut seed) % 2047) as i16 - 1023;
            }
        }
        clamp_block_jpeg_coeffs(&mut b);
        blocks.push(b);
    }

    let encoded = encode_plane(&blocks, false);
    let (decoded, _) = decode_plane(&encoded, 0, n_blocks, false).expect("stress decode failed");
    for (i, (orig, dec)) in blocks.iter().zip(decoded.iter()).enumerate() {
        assert_eq!(orig.data, dec.data, "stress mismatch in block {i}");
    }
}
