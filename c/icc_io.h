#ifndef BITGRAIN_ICC_IO_H
#define BITGRAIN_ICC_IO_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/**
 * Load ICC profile from a PNG file (iCCP chunk).
 * Returns malloc'd ICC data; caller must free(). *out_len receives length.
 * Returns NULL if no ICC or on error.
 */
uint8_t *bitgrain_load_icc_from_png(const char *path, uint32_t *out_len);

/**
 * Write PNG with embedded ICC profile (iCCP chunk).
 * comp: 1 = grayscale, 3 = RGB, 4 = RGBA.
 * icc may be NULL (writes without ICC). Returns 0 on success.
 */
int bitgrain_write_png_with_icc(const char *path,
                                const uint8_t *pixels,
                                uint32_t width,
                                uint32_t height,
                                int comp,
                                const uint8_t *icc,
                                uint32_t icc_len);

#ifdef __cplusplus
}
#endif

#endif
