#ifndef BITGRAIN_WEBP_IO_H
#define BITGRAIN_WEBP_IO_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Load WebP as RGB (width*height*3). Caller frees with bitgrain_image_free(). Returns NULL on error. */
uint8_t *bitgrain_load_webp_rgb(const char *path, uint32_t *out_width, uint32_t *out_height);

/* Load WebP as RGBA (width*height*4). Caller frees with bitgrain_image_free(). Returns NULL on error. */
uint8_t *bitgrain_load_webp_rgba(const char *path, uint32_t *out_width, uint32_t *out_height);

/* Write RGB (comp=3) or RGBA (comp=4) to WebP. quality 1–100. Returns 0 on success. */
int bitgrain_write_webp(const char *path, const uint8_t *pixels,
                        uint32_t width, uint32_t height, int comp, int quality);

#ifdef __cplusplus
}
#endif

#endif
