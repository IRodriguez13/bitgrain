/* SPDX-License-Identifier: GPL-3.0-or-later */
#include "bg_utils.h"
#include "config.h"

int parse_bg_header(const uint8_t *buf, uint32_t *width, uint32_t *height, uint32_t *channels)
{
    if (buf[0] != 'B' || buf[1] != 'G') return -1;
    uint8_t ver = buf[2];
    if (ver < 1 || ver > 5) return -1;
    *width   = (uint32_t)buf[3] | ((uint32_t)buf[4]<<8) | ((uint32_t)buf[5]<<16) | ((uint32_t)buf[6]<<24);
    *height  = (uint32_t)buf[7] | ((uint32_t)buf[8]<<8) | ((uint32_t)buf[9]<<16) | ((uint32_t)buf[10]<<24);
    /* v1=gray(1ch), v2=RGB(3ch), v3=RGBA(4ch), v4=YCbCr420→RGB(3ch), v5=YCbCr420A→RGBA(4ch) */
    switch (ver) {
        case 1: *channels = 1; break;
        case 2: *channels = 3; break;
        case 3: *channels = 4; break;
        case 4: *channels = 3; break;  /* YCbCr420 decodes to RGB */
        case 5: *channels = 4; break;  /* YCbCr420A decodes to RGBA */
        default: return -1;
    }
    return 0;
}

int check_image_size(uint32_t width, uint32_t height, uint32_t channels)
{
    if (width == 0 || height == 0 || width > BITGRAIN_MAX_DIM || height > BITGRAIN_MAX_DIM) return -1;
    uint64_t bytes = (uint64_t)width * height * channels;
    if (bytes > BITGRAIN_MAX_PIXEL_BYTES) return -1;
    return 0;
}
