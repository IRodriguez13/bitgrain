#ifndef BITGRAIN_IMAGE_LOADER_H
#define BITGRAIN_IMAGE_LOADER_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/**
 * Carga una imagen (JPEG, PNG, BMP, PGM, etc.) y la convierte a escala de grises.
 *
 * path: ruta del archivo
 * out_width, out_height: dimensiones de salida
 *
 * Devuelve un buffer width*height bytes (malloc) o NULL si falla.
 * El llamante debe liberar con bitgrain_image_free().
 */
uint8_t *bitgrain_load_grayscale(const char *path,
                                  uint32_t *out_width,
                                  uint32_t *out_height);

void bitgrain_image_free(void *pixels);

#ifdef __cplusplus
}
#endif

#endif
