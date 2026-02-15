/* WebP load/save. Compile only with -DBITGRAIN_USE_WEBP and -lwebp. */

#ifdef BITGRAIN_USE_WEBP

#include "webp_io.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <webp/decode.h>
#include <webp/encode.h>

uint8_t *bitgrain_load_webp_rgb(const char *path, uint32_t *out_width, uint32_t *out_height)
{
    FILE *f = fopen(path, "rb");
   
    if (!f) return NULL;
   
    if (fseek(f, 0, SEEK_END) != 0) { fclose(f); return NULL; }
   
    long sz = ftell(f);
   
    if (sz <= 0 || sz > 256 * 1024 * 1024) { fclose(f); return NULL; }
   
    rewind(f);
   
    uint8_t *raw = (uint8_t *)malloc((size_t)sz);
   
    if (!raw) { fclose(f); return NULL; }
   
    if (fread(raw, 1, (size_t)sz, f) != (size_t)sz) {
        free(raw);
        fclose(f);
        return NULL;
    }
    fclose(f);

    int w, h;
    uint8_t *decoded = WebPDecodeRGB(raw, (size_t)sz, &w, &h);
    free(raw);
    if (!decoded || w <= 0 || h <= 0) return NULL;

    size_t n = (size_t)w * (size_t)h * 3u;
    uint8_t *out = (uint8_t *)malloc(n);
    if (!out) {
        WebPFree(decoded);
        return NULL;
    }
    memcpy(out, decoded, n);
    WebPFree(decoded);
    *out_width = (uint32_t)w;
    *out_height = (uint32_t)h;
    return out;
}

uint8_t *bitgrain_load_webp_rgba(const char *path, uint32_t *out_width, uint32_t *out_height)
{
    FILE *f = fopen(path, "rb");
    if (!f) return NULL;
    if (fseek(f, 0, SEEK_END) != 0) { fclose(f); return NULL; }
    long sz = ftell(f);
    if (sz <= 0 || sz > 256 * 1024 * 1024) { fclose(f); return NULL; }
    rewind(f);
    uint8_t *raw = (uint8_t *)malloc((size_t)sz);
    if (!raw) { fclose(f); return NULL; }
    if (fread(raw, 1, (size_t)sz, f) != (size_t)sz) {
        free(raw);
        fclose(f);
        return NULL;
    }
    fclose(f);

    int w, h;
    uint8_t *decoded = WebPDecodeRGBA(raw, (size_t)sz, &w, &h);
    free(raw);
    if (!decoded || w <= 0 || h <= 0) return NULL;

    size_t n = (size_t)w * (size_t)h * 4u;
    uint8_t *out = (uint8_t *)malloc(n);
    if (!out) {
        WebPFree(decoded);
        return NULL;
    }
    memcpy(out, decoded, n);
    WebPFree(decoded);
    *out_width = (uint32_t)w;
    *out_height = (uint32_t)h;
    return out;
}

int bitgrain_write_webp(const char *path, const uint8_t *pixels,
                        uint32_t width, uint32_t height, int comp, int quality)
{
    if (!path || !pixels || width == 0 || height == 0) return -1;
    if (quality < 1) quality = 80;
    if (quality > 100) quality = 100;
    float q = (float)quality;

    uint8_t *out = NULL;
    size_t out_size = 0;
    if (comp == 4) {
        out_size = WebPEncodeRGBA(pixels, (int)width, (int)height, (int)width * 4, q, &out);
    } else {
        out_size = WebPEncodeRGB(pixels, (int)width, (int)height, (int)width * 3, q, &out);
    }
    if (!out || out_size == 0) return -1;

    FILE *f = fopen(path, "wb");
    if (!f) {
        WebPFree(out);
        return -1;
    }
    int ok = (fwrite(out, 1, out_size, f) == out_size);
    fclose(f);
    WebPFree(out);
    return ok ? 0 : -1;
}

#endif
