/* SPDX-License-Identifier: GPL-3.0-or-later  Copyright (C) 2026 Iván E. Rodriguez */
#include<array.h>
#include<stdint.h>

namespace bitgrain::core{
    constexpr int BLOCK_SIZE = 8;
    constexpr int BLOCK_AREA = 64;

    class Block{
        public:
            int16_t& at(int x, int y);
            const int16_t& at(int x, int y) const;

        private:
            std::array<int16_t, BLOCK_AREA>data_{};
    }
}
