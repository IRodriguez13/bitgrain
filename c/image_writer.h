#ifndef BITGRAIN_IMAGE_WRITER_H
#define BITGRAIN_IMAGE_WRITER_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/**
 * Escribe una imagen en escala de grises como JPEG.
 * pixels: width*height bytes, orden filas.
 * quality: 1-100 (típico 80-90).
 * Devuelve 0 si ok, distinto de 0 si error.
 */
int bitgrain_write_jpg(const char *path,
                       const uint8_t *pixels,
                       uint32_t width,
                       uint32_t height,
                       int quality);

#ifdef __cplusplus
}
#endif

#endif
