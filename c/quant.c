#include "quant.h"

void quantize_block(int16_t* block, const int16_t* table)
{
    for (int i = 0; i < 64; i++)
    {
        block[i] = block[i] / table[i];
    }
}
