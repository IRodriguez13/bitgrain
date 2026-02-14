use crate::block::Block;
use std::f64::consts::PI;

pub fn dct(block: &mut Block) {
    let mut result = [0f64; 64];

    for u in 0..8 {
        for v in 0..8 {

            let mut sum = 0.0;

            for x in 0..8 {
                for y in 0..8 {

                    let pixel = block.data[y * 8 + x] as f64;

                    sum += pixel *
                        ((2 * x + 1) as f64 * u as f64 * PI / 16.0).cos() *
                        ((2 * y + 1) as f64 * v as f64 * PI / 16.0).cos();
                }
            }

            let cu = if u == 0 { 1.0 / 2f64.sqrt() } else { 1.0 };
            let cv = if v == 0 { 1.0 / 2f64.sqrt() } else { 1.0 };

            result[v * 8 + u] = 0.25 * cu * cv * sum;
        }
    }

    for i in 0..64 {
        block.data[i] = result[i].round() as i16;
    }
}

/// IDCT (inverse of DCT): coefficients → centered pixels (-128..127).
pub fn idct(block: &mut Block) {
    let mut result = [0f64; 64];

    for y in 0..8 {
        for x in 0..8 {
            let mut sum = 0.0;

            for u in 0..8 {
                for v in 0..8 {
                    let coef = block.data[v * 8 + u] as f64;
                    let cu = if u == 0 { 1.0 / 2f64.sqrt() } else { 1.0 };
                    let cv = if v == 0 { 1.0 / 2f64.sqrt() } else { 1.0 };

                    sum += cu * cv * coef
                        * ((2 * x + 1) as f64 * u as f64 * PI / 16.0).cos()
                        * ((2 * y + 1) as f64 * v as f64 * PI / 16.0).cos();
                }
            }

            result[y * 8 + x] = 0.25 * sum;
        }
    }

    for i in 0..64 {
        block.data[i] = result[i].round() as i16;
    }
}

