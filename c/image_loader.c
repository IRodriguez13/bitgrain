#include "image_loader.h"
#include "webp_io.h"
#include <stdlib.h>
#include <string.h>
#include <strings.h>

#define STB_IMAGE_IMPLEMENTATION
#define STB_IMAGE_STATIC
#include "stb_image.h"

uint8_t *bitgrain_load_grayscale(const char *path,
                                  uint32_t *out_width,
                                  uint32_t *out_height)
{
    int w, h, n;
    unsigned char *data = stbi_load(path, &w, &h, &n, 1); /* 1 = 1 channel (gray); RGB converted automatically */
    if (!data) {
        (void)n;
        return NULL;
    }
    if (w <= 0 || h <= 0) {
        stbi_image_free(data);
        return NULL;
    }
    *out_width  = (uint32_t)w;
    *out_height = (uint32_t)h;
    return (uint8_t *)data;
}

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
    if (!data) {
        (void)n;
        return NULL;
    }
    if (w <= 0 || h <= 0) {
        stbi_image_free(data);
        return NULL;
    }
    *out_width  = (uint32_t)w;
    *out_height = (uint32_t)h;
    return (uint8_t *)data;
}

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
    if (!data) return NULL;
    if (w <= 0 || h <= 0) {
        stbi_image_free(data);
        return NULL;
    }
    *out_width  = (uint32_t)w;
    *out_height = (uint32_t)h;
    return (uint8_t *)data;
}

void bitgrain_image_free(void *pixels)
{
    stbi_image_free(pixels);
}
