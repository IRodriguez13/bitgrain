/* SPDX-License-Identifier: GPL-3.0-or-later */
#ifndef BITGRAIN_BG_UTILS_H
#define BITGRAIN_BG_UTILS_H

#include <stdint.h>

/* Parse .bg header (11 bytes). Sets *channels (1, 3, or 4). Returns 0 on success. */
int parse_bg_header(const uint8_t *buf, uint32_t *width, uint32_t *height, uint32_t *channels);

/* Check image dimensions against limits. Returns 0 if OK. */
int check_image_size(uint32_t width, uint32_t height, uint32_t channels);

#endif
