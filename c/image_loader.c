#include "image_loader.h"
#include "webp_io.h"
#include <stdlib.h>
#include <string.h>
#include <strings.h>

#define STB_IMAGE_IMPLEMENTATION
#define STB_IMAGE_STATIC
#if defined(__GNUC__)
#pragma GCC diagnostic push
#pragma GCC diagnostic ignored "-Wunused-function"
#endif
#include "stb_image.h"
#if defined(__GNUC__)
#pragma GCC diagnostic pop
#endif

/* ------------------------------------------------------------------ */
/* Stream reader                                                        */
/* ------------------------------------------------------------------ */

uint8_t *bitgrain_read_stream(FILE *f, size_t *out_size)
{
    size_t cap = 65536;
    size_t len = 0;
    uint8_t *buf = (uint8_t *)malloc(cap);
    if (!buf) return NULL;

    size_t n;
    while ((n = fread(buf + len, 1, cap - len, f)) > 0) {
        len += n;
        if (len == cap) {
            cap *= 2;
            uint8_t *tmp = (uint8_t *)realloc(buf, cap);
            if (!tmp) { free(buf); return NULL; }
            buf = tmp;
        }
    }
    *out_size = len;
    return buf;
}

/* ------------------------------------------------------------------ */
/* Grayscale                                                            */
/* ------------------------------------------------------------------ */

uint8_t *bitgrain_load_grayscale(const char *path,
                                  uint32_t *out_width,
                                  uint32_t *out_height)
{
    int w, h, n;
    unsigned char *data = stbi_load(path, &w, &h, &n, 1);
    if (!data || w <= 0 || h <= 0) { stbi_image_free(data); return NULL; }
    *out_width  = (uint32_t)w;
    *out_height = (uint32_t)h;
    return (uint8_t *)data;
}

uint8_t *bitgrain_load_grayscale_mem(const uint8_t *mem, size_t mem_size,
                                      uint32_t *out_width, uint32_t *out_height)
{
    int w, h, n;
    unsigned char *data = stbi_load_from_memory(mem, (int)mem_size, &w, &h, &n, 1);
    if (!data || w <= 0 || h <= 0) { stbi_image_free(data); return NULL; }
    *out_width  = (uint32_t)w;
    *out_height = (uint32_t)h;
    return (uint8_t *)data;
}

/* ------------------------------------------------------------------ */
/* RGB                                                                  */
/* ------------------------------------------------------------------ */

uint8_t *bitgrain_load_rgb(const char *path,
                           uint32_t *out_width,
                           uint32_t *out_height)
{
    const char *dot = strrchr(path, '.');
    if (dot && strcasecmp(dot, ".webp") == 0) {
        uint8_t *data = bitgrain_load_webp_rgb(path, out_width, out_height);
        if (data) return data;
    }
    int w, h, n;
    unsigned char *data = stbi_load(path, &w, &h, &n, 3);
    if (!data || w <= 0 || h <= 0) { stbi_image_free(data); return NULL; }
    *out_width  = (uint32_t)w;
    *out_height = (uint32_t)h;
    return (uint8_t *)data;
}

uint8_t *bitgrain_load_rgb_mem(const uint8_t *mem, size_t mem_size,
                                uint32_t *out_width, uint32_t *out_height)
{
    int w, h, n;
    unsigned char *data = stbi_load_from_memory(mem, (int)mem_size, &w, &h, &n, 3);
    if (!data || w <= 0 || h <= 0) { stbi_image_free(data); return NULL; }
    *out_width  = (uint32_t)w;
    *out_height = (uint32_t)h;
    return (uint8_t *)data;
}

/* ------------------------------------------------------------------ */
/* RGBA                                                                 */
/* ------------------------------------------------------------------ */

uint8_t *bitgrain_load_rgba(const char *path,
                            uint32_t *out_width,
                            uint32_t *out_height)
{
    const char *dot = strrchr(path, '.');
    if (dot && strcasecmp(dot, ".webp") == 0) {
        uint8_t *data = bitgrain_load_webp_rgba(path, out_width, out_height);
        if (data) return data;
    }
    int w, h, n;
    unsigned char *data = stbi_load(path, &w, &h, &n, 4);
    if (!data || w <= 0 || h <= 0) { stbi_image_free(data); return NULL; }
    *out_width  = (uint32_t)w;
    *out_height = (uint32_t)h;
    return (uint8_t *)data;
}

uint8_t *bitgrain_load_rgba_mem(const uint8_t *mem, size_t mem_size,
                                 uint32_t *out_width, uint32_t *out_height)
{
    int w, h, n;
    unsigned char *data = stbi_load_from_memory(mem, (int)mem_size, &w, &h, &n, 4);
    if (!data || w <= 0 || h <= 0) { stbi_image_free(data); return NULL; }
    *out_width  = (uint32_t)w;
    *out_height = (uint32_t)h;
    return (uint8_t *)data;
}

void bitgrain_image_free(void *pixels)
{
    stbi_image_free(pixels);
}
