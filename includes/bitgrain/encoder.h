/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * Copyright (C) 2026 Iván E. Rodríguez
 */

#pragma once

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif


typedef struct bitgrain_encoder bitgrain_encoder_t;

typedef enum {
  GRAIN_SUBSAMPLING_444 = 0,
  GRAIN_SUBSAMPLING_422 = 1,
  GRAIN_SUBSAMPLING_420 = 2
} bitgrain_chroma_subsampling_t;

typedef struct {
  uint8_t quality; /* 1..100 */
  grain_chroma_subsampling_t chroma_subsampling;
} bitgrain_encoder_config_t;


bitgrain_encoder_config_t* bitgrain_encoder_create(
  const bitgrain_encoder_config_t* config
);

void bitgrain_encoder_destroy(
  bitgrain_encoder_t* encoder
);

#ifdef __cplusplus
}
#endif
