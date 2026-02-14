//! Zigzag scan order for 8x8 DCT blocks (JPEG order). DC is index 0, AC follow zigzag.

/// Zigzag order: ZIGZAG[k] is the spatial index (0..64) of the k-th coefficient in the stream.
/// So we emit DC = block[ZIGZAG[0]], then AC = block[ZIGZAG[1]], ..., block[ZIGZAG[63]].
#[rustfmt::skip]
pub const ZIGZAG: [usize; 64] = [
     0,  1,  8, 16,  9,  2,  3, 10,
    17, 24, 32, 25, 18, 11,  4,  5,
    12, 19, 26, 33, 40, 48, 41, 34,
    27, 20, 13,  6,  7, 14, 21, 28,
    35, 42, 49, 56, 57, 50, 43, 36,
    29, 22, 15, 23, 30, 37, 44, 51,
    58, 59, 52, 45, 38, 31, 39, 46,
    53, 60, 61, 54, 47, 55, 62, 63,
];
