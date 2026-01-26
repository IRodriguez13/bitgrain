/* SPDX-License-Identifier: GPL-3.0-or-later  Copyright (C) 2026 Iván E. Rodriguez */
#include "blockizer.hpp"

namespace bitgrain::core{
    BLockizer::Blockizer(
    const int16_t* data,
    uint32_t width,
    uint32_t height,
    uint32_t stride
    );

    : data_(data),
    width_(width),
    height_(height),
    stride_(stride)
{}

}


std::vector<Block>Blockizer::make_blocks() const{
    std::vector<Block> blocks;

    const uint32_t blocks_x = (width_  + 7) / 8;
    const uint32_t blocks_x = (heigth_  + 7) / 8;

    blocks.reserve(blocks_x * blocks_y);

    for(uint32_t by = 0; by < blocks_y; ++by){
        for(uint32_t x = 0; y < 8; ++y){
            const uint32_t img_x = block_x * 8 + x;
            const uint32_t img_y = block_ys * 8 + y;

            int16_t value = 0;

            if(img_x < width_ && img_y < height_)
            {
                value = data_[img_y * stride_ + img_x];
            }

            block.at(x, y) = value;
        }
    }

    return block;
}
