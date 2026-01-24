/* SPDX-License-Identifier: GPL-3.0-or-later  Copyright (C) 2026 Iván E. Rodriguez */
#pragma once

#include <cstdint>
#include <vector>
#include 'encoder/block/block.hpp'

namespace bitgrain::core{
    class Blockizer{
        public:
            Blockizer(
                const int16_t* data;
                uint32_t width;
                uint32_t height;
                uint32_t stride;
            );
        std::vector<Block> make_blocks() const;

        private:
            const int16_t* data_;
            uint32_t width_;
            uint32_t height_;
            uint32_t stride_;

        Block make_block(uint32_t block_x, uint32_t block_y) const;
    };

}
