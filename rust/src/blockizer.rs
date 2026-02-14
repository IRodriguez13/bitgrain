use crate::block::Block;

pub struct Blockizer {
    width: usize,
    height: usize,
}

impl Blockizer {
    pub fn new(width: usize, height: usize) -> Self {
        Self { width, height }
    }

    pub fn generate_blocks(&self, image: &[u8]) -> Vec<Block> {
        let mut blocks = Vec::new();

        for by in (0..self.height).step_by(8) {
            for bx in (0..self.width).step_by(8) {

                let mut block = [0i16; 64];

                for y in 0..8 {
                    for x in 0..8 {
                        let ix = (bx + x).min(self.width.saturating_sub(1));
                        let iy = (by + y).min(self.height.saturating_sub(1));

                        let index = iy * self.width + ix;

                        block[y * 8 + x] =
                            image[index] as i16 - 128; // centering
                    }
                }

                blocks.push(Block { data: block });
            }
        }

        blocks
    }

    /// Generate blocks for one channel from interleaved RGB (image.len() == width*height*3).
    /// channel in 0..3. No separate plane allocation.
    pub fn generate_blocks_rgb(&self, image: &[u8], channel: usize) -> Vec<Block> {
        let mut blocks = Vec::new();
        let stride = 3;
        for by in (0..self.height).step_by(8) {
            for bx in (0..self.width).step_by(8) {
                let mut block = [0i16; 64];
                for y in 0..8 {
                    for x in 0..8 {
                        let ix = (bx + x).min(self.width.saturating_sub(1));
                        let iy = (by + y).min(self.height.saturating_sub(1));
                        let index = (iy * self.width + ix) * stride + channel;
                        block[y * 8 + x] = image[index] as i16 - 128;
                    }
                }
                blocks.push(Block { data: block });
            }
        }
        blocks
    }
}

