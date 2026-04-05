use crate::block::Block;
use rayon::prelude::*;

pub struct Blockizer {
    width: usize,
    height: usize,
}

impl Blockizer {
    pub fn new(width: usize, height: usize) -> Self {
        Self { width, height }
    }

    /// Generate 8×8 blocks from a grayscale plane. Parallel via Rayon.
    pub fn generate_blocks(&self, image: &[u8]) -> Vec<Block> {
        let w = self.width;
        let h = self.height;
        let blocks_wide = (w + 7) / 8;
        let blocks_high = (h + 7) / 8;
        let num_blocks = blocks_wide * blocks_high;

        (0..num_blocks)
            .into_par_iter()
            .map(|idx| {
                let bx = (idx % blocks_wide) * 8;
                let by = (idx / blocks_wide) * 8;
                let mut block = [0i16; 64];
                for y in 0..8 {
                    let iy = (by + y).min(h.saturating_sub(1));
                    let row_base = iy * w;
                    for x in 0..8 {
                        let ix = (bx + x).min(w.saturating_sub(1));
                        block[y * 8 + x] = image[row_base + ix] as i16 - 128;
                    }
                }
                Block { data: block }
            })
            .collect()
    }

    /// Generate blocks for one channel from interleaved RGB. Parallel via Rayon.
    pub fn generate_blocks_rgb(&self, image: &[u8], channel: usize) -> Vec<Block> {
        self.generate_blocks_interleaved_par(image, 3, channel)
    }

    /// Generate blocks for one channel from interleaved RGBA. Parallel via Rayon.
    pub fn generate_blocks_rgba(&self, image: &[u8], channel: usize) -> Vec<Block> {
        self.generate_blocks_interleaved_par(image, 4, channel)
    }

    fn generate_blocks_interleaved_par(&self, image: &[u8], stride: usize, channel: usize) -> Vec<Block> {
        let w = self.width;
        let h = self.height;
        let blocks_wide = (w + 7) / 8;
        let blocks_high = (h + 7) / 8;
        let num_blocks = blocks_wide * blocks_high;

        (0..num_blocks)
            .into_par_iter()
            .map(|idx| {
                let bx = (idx % blocks_wide) * 8;
                let by = (idx / blocks_wide) * 8;
                let mut block = [0i16; 64];
                for y in 0..8 {
                    let iy = (by + y).min(h.saturating_sub(1));
                    let row_base = iy * w;
                    for x in 0..8 {
                        let ix = (bx + x).min(w.saturating_sub(1));
                        block[y * 8 + x] = image[(row_base + ix) * stride + channel] as i16 - 128;
                    }
                }
                Block { data: block }
            })
            .collect()
    }
}
