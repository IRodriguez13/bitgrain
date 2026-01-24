/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * Copyright (C) 2026 Iván E. Rodríguez
 */

#pragma once

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif


typedef struct encoder encoder_t;

typedef enum {
  SUBSAMPLING_444 = 0,
  SUBSAMPLING_422 = 1,
  SUBSAMPLING_420 = 2
} chroma_subsampling_t chroma_subsampling;

typedef struct {
  uint8_t quality; /* 1..100 */
  chroma_subsampling_t chroma_subsampling;
} encoder_config_t;


encoder_t* encoder_create(
  const encoder_config_t* config
);

void encoder_destroy(
  encoder_t* encoder
);

#ifdef __cplusplus
}
#endif
