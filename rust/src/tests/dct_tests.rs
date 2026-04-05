use crate::block::Block;
use crate::dct::{dct, dct_reference, idct, idct_reference};

fn block_from(s: &[i16; 64]) -> Block {
    Block { data: *s }
}

#[test]
fn dct_matches_reference() {
    let input = [
        -128i16, 0, 1, 2, 3, 4, 5, 6,
           7, 8, 9,10,11,12,13,14,
          15,16,17,18,19,20,21,22,
          23,24,25,26,27,28,29,30,
          31,32,33,34,35,36,37,38,
          39,40,41,42,43,44,45,46,
          47,48,49,50,51,52,53,54,
          55,56,57,58,59,60,61,62,
    ];
    let mut block = block_from(&input);
    let expected = dct_reference(&block);
    dct(&mut block);
    for i in 0..64 {
        assert_eq!(block.data[i], expected[i], "dct mismatch at {i}");
    }
}

#[test]
fn idct_matches_reference() {
    let coef: [i16; 64] = [
        0,100,0,0,0,0,0,0,
        0,  0,0,0,0,0,0,0,
        0,  0,0,0,0,0,0,0,
        0,  0,0,0,0,0,0,0,
        0,  0,0,0,0,0,0,0,
        0,  0,0,0,0,0,0,0,
        0,  0,0,0,0,0,0,0,
        0,  0,0,0,0,0,0,0,
    ];
    let expected = idct_reference(&coef);
    let mut block = block_from(&coef);
    idct(&mut block);
    for i in 0..64 {
        assert_eq!(block.data[i], expected[i], "idct mismatch at {i}");
    }
}

#[test]
fn roundtrip_dct_idct() {
    let mut block = block_from(&[
        -50, 10, 20, 30, 40, 50, 60, 70,
         80, 90,100,110,120,127,127,127,
          0,-10,-20,-30,-40,-50,-60,-70,
          1,  2,  3,  4,  5,  6,  7,  8,
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
        assert!(diff <= 1, "roundtrip diff at {i}: {} vs {}", block.data[i], original[i]);
    }
}

#[test]
fn reference_roundtrip() {
    let input: [i16; 64] = core::array::from_fn(|i| (i as i16 * 3) - 96);
    let block = Block { data: input };
    let dct_out = dct_reference(&block);
    let idct_out = idct_reference(&dct_out);
    for i in 0..64 {
        let diff = (idct_out[i] - input[i]).abs();
        assert!(diff <= 1, "reference roundtrip diff at {i}: {} vs {}", idct_out[i], input[i]);
    }
}
