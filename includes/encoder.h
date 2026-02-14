#ifndef BITGRAIN_ENCODER_H
#define BITGRAIN_ENCODER_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/**
 * Codifica una imagen en escala de grises (8 bpp) a flujo .bg.
 *
 * image: puntero a width*height bytes (orden filas, 1 byte por píxel)
 * out_buffer: buffer donde se escribe el flujo comprimido (con cabecera .bg)
 * out_capacity: tamaño máximo de out_buffer en bytes
 * out_len: salida, número de bytes escritos
 *
 * Devuelve 0 si ok, -1 si error.
 */
int bitgrain_encode_grayscale(
    const uint8_t *image,
    uint32_t width,
    uint32_t height,
    uint8_t *out_buffer,
    uint32_t out_capacity,
    int32_t *out_len);

/**
 * Decodifica un flujo .bg en imagen en escala de grises.
 *
 * buffer: flujo .bg completo (incluye cabecera)
 * size: número de bytes de buffer
 * out_pixels: buffer de salida (se escriben width*height bytes)
 * out_capacity: tamaño de out_pixels (debe ser >= width*height tras decodificar)
 * out_width, out_height: salida, dimensiones de la imagen
 *
 * Devuelve 0 si ok, -1 si error.
 */
int bitgrain_decode_grayscale(
    const uint8_t *buffer,
    int32_t size,
    uint8_t *out_pixels,
    uint32_t out_capacity,
    uint32_t *out_width,
    uint32_t *out_height);

#ifdef __cplusplus
}
#endif

#endif
