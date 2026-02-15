#ifndef BITGRAIN_DCT_H
#define BITGRAIN_DCT_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

void bitgrain_dct_block(int16_t *block);
void bitgrain_idct_block(int16_t *block);

#ifdef __cplusplus
}
#endif

#endif
