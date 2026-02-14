#ifndef BITGRAIN_ENCODER_H
#define BITGRAIN_ENCODER_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/**
 * Encode a grayscale image (8 bpp) to .bg stream.
 *
 * image: pointer to width*height bytes (row order, 1 byte per pixel)
 * out_buffer: buffer where compressed stream is written (with .bg header)
 * out_capacity: maximum size of out_buffer in bytes
 * out_len: output, number of bytes written
 *
 * Returns 0 on success, -1 on error.
 */
int bitgrain_encode_grayscale(
    const uint8_t *image,
    uint32_t width,
    uint32_t height,
    uint8_t *out_buffer,
    uint32_t out_capacity,
    int32_t *out_len);

/**
 * Encode an RGB image (24 bpp, R G B per pixel) to .bg stream.
 * image: width*height*3 bytes.
 */
int bitgrain_encode_rgb(
    const uint8_t *image,
    uint32_t width,
    uint32_t height,
    uint8_t *out_buffer,
    uint32_t out_capacity,
    int32_t *out_len);

/**
 * Decode a .bg stream into pixels (grayscale or RGB per header).
 * out_channels: output, 1 = grayscale (w*h bytes), 3 = RGB (w*h*3 bytes).
 * out_capacity must be >= width*height*out_channels.
 */
int bitgrain_decode(
    const uint8_t *buffer,
    int32_t size,
    uint8_t *out_pixels,
    uint32_t out_capacity,
    uint32_t *out_width,
    uint32_t *out_height,
    uint32_t *out_channels);

/**
 * Decode a .bg stream to grayscale (version 1 only).
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
