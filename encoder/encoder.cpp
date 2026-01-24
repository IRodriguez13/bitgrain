/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * Copyright (C) 2026 Iván E. Rodríguez
 */

#include "bitgrain/encoder.h"
#include <new>      // std::nothrow
#include <cstdint>


namespace bitgrain::jpeg
{
    class Encoder;
    /* Wrapper C */
}

struct bitgrain_encoder
{
  bitgrain::jpeg::Encoder* impl;
};

bitgrain_encoder_t* bitgrain_encoder_create(
  const bitgrain_encoder_config_t* config
) {
  if (!config)
  {
    return nullptr;
  }

  bitgrain_encoder_t* enc =
    new (std::nothrow) bitgrain_encoder_t{};

  if (!enc)
  {
    return nullptr;
  }

  enc->impl = new (std::nothrow)
    bitgrain::jpeg::Encoder(*config);

  if (!enc->impl)
  {
    delete enc;
    return nullptr;
  }

  return enc;
}

void bitgrain_encoder_destroy(
  bitgrain_encoder_t* encoder
) {
  if (!encoder) {
    return;
  }

  delete encoder->impl;
  delete encoder;
}
