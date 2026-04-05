//! DCT/IDCT for 8×8 blocks.
//! In release builds, delegates to C SIMD (c/dct.c: SSE2/NEON/scalar dispatch).
//! In test builds and as a pure-Rust fallback, uses the reference f64 implementation.

use crate::block::Block;
use std::f64::consts::PI;

/// Pure-Rust reference forward DCT. Used in tests and as a software fallback.
/// Matches the separable DCT-II definition used by the C implementation.
pub fn dct_reference(block: &Block) -> [i16; 64] {
    let mut result = [0f64; 64];
    for u in 0..8 {
        for v in 0..8 {
            let mut sum = 0.0f64;
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

/// Pure-Rust reference inverse DCT. Used in tests and as a software fallback.
pub fn idct_reference(coef: &[i16; 64]) -> [i16; 64] {
    let mut result = [0f64; 64];
    for y in 0..8 {
        for x in 0..8 {
            let mut sum = 0.0f64;
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

/// Forward 8×8 DCT.
/// Release: delegates to C SIMD (SSE2/NEON/scalar selected at compile time in c/dct.c).
/// Test: uses the pure-Rust reference implementation so tests run without the C lib.
#[inline]
pub fn dct(block: &mut Block) {
    #[cfg(not(test))]
    unsafe { crate::ffi::bitgrain_dct_block(block.data.as_mut_ptr()) }

    #[cfg(test)]
    { block.data = dct_reference(block); }
}

/// Inverse 8×8 DCT. Coefficients → centered pixels (-128..127).
/// Release: delegates to C SIMD. Test: pure-Rust reference.
#[inline]
pub fn idct(block: &mut Block) {
    #[cfg(not(test))]
    unsafe { crate::ffi::bitgrain_idct_block(block.data.as_mut_ptr()) }

    #[cfg(test)]
    { block.data = idct_reference(&block.data); }
}

