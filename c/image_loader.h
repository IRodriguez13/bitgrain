#ifndef BITGRAIN_IMAGE_LOADER_H
#define BITGRAIN_IMAGE_LOADER_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/**
 * Load an image and convert to grayscale.
 * Returns width*height bytes. Caller must free with bitgrain_image_free().
 */
uint8_t *bitgrain_load_grayscale(const char *path,
                                  uint32_t *out_width,
                                  uint32_t *out_height);

/**
 * Load an image as RGB (3 channels per pixel, R G B order).
 * Returns width*height*3 bytes. Caller must free with bitgrain_image_free().
 */
uint8_t *bitgrain_load_rgb(const char *path,
                           uint32_t *out_width,
                           uint32_t *out_height);

/**
 * Load an image as RGBA (4 channels, R G B A order). Opaque if no alpha in file.
 * Returns width*height*4 bytes. Caller must free with bitgrain_image_free().
 */
uint8_t *bitgrain_load_rgba(const char *path,
                            uint32_t *out_width,
                            uint32_t *out_height);

void bitgrain_image_free(void *pixels);

#ifdef __cplusplus
}
#endif

#endif
