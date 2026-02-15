#ifndef BITGRAIN_IMAGE_WRITER_H
#define BITGRAIN_IMAGE_WRITER_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/**
 * Write an image as JPEG.
 * comp: 1 = grayscale, 3 = RGB. quality: 1-100.
 * Returns 0 on success, non-zero on error.
 */
int bitgrain_write_jpg(const char *path,
                       const uint8_t *pixels,
                       uint32_t width,
                       uint32_t height,
                       int comp,
                       int quality);

/**
 * Write an image as PNG (lossless).
 * comp: 1 = grayscale, 3 = RGB, 4 = RGBA.
 * Returns 0 on success, non-zero on error.
 */
int bitgrain_write_png(const char *path,
                       const uint8_t *pixels,
                       uint32_t width,
                       uint32_t height,
                       int comp);

/**
 * Write an image as WebP. comp: 3 = RGB, 4 = RGBA. quality 1–100.
 * Returns 0 on success, non-zero on error.
 */
int bitgrain_write_webp(const char *path,
                        const uint8_t *pixels,
                        uint32_t width,
                        uint32_t height,
                        int comp,
                        int quality);

/**
 * Write grayscale as PGM (P5). comp must be 1.
 * Returns 0 on success.
 */
int bitgrain_write_pgm(const char *path,
                       const uint8_t *pixels,
                       uint32_t width,
                       uint32_t height);

#ifdef __cplusplus
}
#endif

#endif
