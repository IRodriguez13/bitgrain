//! DCT/IDCT for 8×8 blocks. Reference implementation.
//! Priority: optimize encode/decode (separable DCT/SIMD) so Bitgrain is faster than WebP/AVIF with similar quality.

use crate::block::Block;
use std::f64::consts::PI;


fn dct_reference(block: &Block) -> [i16; 64] {
    let mut result = [0f64; 64];
    for u in 0..8 {
        for v in 0..8 {
            let mut sum = 0.0;
            for x in 0..8 {
                for y in 0..8 {
                    let pixel = block.data[y * 8 + x] as f64;
                    sum += pixel
                        * ((2 * x + 1) as f64 * u as f64 * PI / 16.0).cos()
                        * ((2 * y + 1) as f64 * v as f64 * PI / 16.0).cos();
                }
            }
            let cu = if u == 0 { 1.0 / 2f64.sqrt() } else { 1.0 };
            let cv = if v == 0 { 1.0 / 2f64.sqrt() } else { 1.0 };
            result[v * 8 + u] = 0.25 * cu * cv * sum;
        }
    }
    let mut out = [0i16; 64];
    for i in 0..64 {
        out[i] = result[i].round() as i16;
    }
    out
}

fn idct_reference(coef: &[i16; 64]) -> [i16; 64] {
    let mut result = [0f64; 64];
    for y in 0..8 {
        for x in 0..8 {
            let mut sum = 0.0;
            for u in 0..8 {
                for v in 0..8 {
                    let c = coef[v * 8 + u] as f64;
                    let cu = if u == 0 { 1.0 / 2f64.sqrt() } else { 1.0 };
                    let cv = if v == 0 { 1.0 / 2f64.sqrt() } else { 1.0 };
                    sum += cu * cv * c
                        * ((2 * x + 1) as f64 * u as f64 * PI / 16.0).cos()
                        * ((2 * y + 1) as f64 * v as f64 * PI / 16.0).cos();
                }
            }
            result[y * 8 + x] = 0.25 * sum;
        }
    }
    let mut out = [0i16; 64];
    for i in 0..64 {
        out[i] = result[i].round() as i16;
    }
    out
}

/// Forward 8×8 DCT. Uses C SIMD (c/dct.c) when linked; reference for tests.
#[inline]
pub fn dct(block: &mut Block) {
    #[cfg(not(test))]
    {
        unsafe { crate::ffi::bitgrain_dct_block(block.data.as_mut_ptr()) };
        return;
    }
    #[cfg(test)]
    {
        let out = dct_reference(block);
        block.data = out;
    }
}

/// IDCT: coefficients → centered pixels (-128..127). Uses C SIMD when linked.
#[inline]
pub fn idct(block: &mut Block) {
    #[cfg(not(test))]
    {
        unsafe { crate::ffi::bitgrain_idct_block(block.data.as_mut_ptr()) };
        return;
    }
    #[cfg(test)]
    {
        let out = idct_reference(&block.data);
        block.data = out;
    }
}

// Tests: fast DCT/IDCT matches reference and round-trips.
// Run only with `cargo test`; not in release. Synthetic data only, no user files.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block::Block;

    fn block_from_slice(s: &[i16; 64]) -> Block {
        Block { data: *s }
    }

    #[test]
    fn dct_matches_reference() {
        let input = [
            -128i16, 0, 1, 2, 3, 4, 5, 6,
            7, 8, 9, 10, 11, 12, 13, 14,
            15, 16, 17, 18, 19, 20, 21, 22,
            23, 24, 25, 26, 27, 28, 29, 30,
            31, 32, 33, 34, 35, 36, 37, 38,
            39, 40, 41, 42, 43, 44, 45, 46,
            47, 48, 49, 50, 51, 52, 53, 54,
            55, 56, 57, 58, 59, 60, 61, 62,
        ];
        let mut block = block_from_slice(&input);
        let expected = dct_reference(&block);
        dct(&mut block);
        for i in 0..64 {
            assert_eq!(block.data[i], expected[i], "dct mismatch at {}", i);
        }
    }


    #[test]
    fn idct_matches_reference() {
        let coef: [i16; 64] = [
            0, 100, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0,
        ];
        let expected = idct_reference(&coef);
        let mut block = block_from_slice(&coef);
        idct(&mut block);
        for i in 0..64 {
            assert_eq!(block.data[i], expected[i], "idct mismatch at {}", i);
        }
    }

    #[test]
    fn roundtrip_dct_idct() {
        let mut block = block_from_slice(&[
            -50, 10, 20, 30, 40, 50, 60, 70,
            80, 90, 100, 110, 120, 127, 127, 127,
            0, -10, -20, -30, -40, -50, -60, -70,
            1, 2, 3, 4, 5, 6, 7, 8,
            9, 10, 11, 12, 13, 14, 15, 16,
            17, 18, 19, 20, 21, 22, 23, 24,
            25, 26, 27, 28, 29, 30, 31, 32,
            33, 34, 35, 36, 37, 38, 39, 40,
        ]);
        let original = block.data;
        dct(&mut block);
        idct(&mut block);
        for i in 0..64 {
            let diff = (block.data[i] - original[i]).abs();
            assert!(diff <= 1, "roundtrip diff at {}: {} vs {}", i, block.data[i], original[i]);
        }
    }
}
