/* SPDX-License-Identifier: GPL-3.0-or-later  Copyright (C) 2026 Iván E. Rodriguez */
#include "block.hpp"
#include <stdexcept>

nampespace bitgrain::core{

    int16_t& Block::at(int x, int y)
    {
        return data_[y * BLOCK_SIZE + x];
    }

    const int16_t& Block::at(int x, int y) const
    {
        return data_ [y * BLOCK_SIZE + x];
    }
}
