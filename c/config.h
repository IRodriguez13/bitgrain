/* SPDX-License-Identifier: GPL-3.0-or-later */
#ifndef BITGRAIN_CONFIG_H
#define BITGRAIN_CONFIG_H

#include <stdint.h>

/* Shared limits for C and Rust. Keep in sync with decoder.rs. */
#define BITGRAIN_MAX_DIM         65536
#define BITGRAIN_MAX_PIXEL_BYTES (2ULL * 1024 * 1024 * 1024)
#define BITGRAIN_MAX_BG_FILE     (2ULL * 1024 * 1024 * 1024)
#define BITGRAIN_OUT_BUF_MARGIN  (1024 * 1024)

#endif
