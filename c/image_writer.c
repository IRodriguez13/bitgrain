#include "image_writer.h"

#define STB_IMAGE_WRITE_IMPLEMENTATION
#define STB_IMAGE_WRITE_STATIC
#include "stb_image_write.h"

#include <stdlib.h>
#include <string.h>

int bitgrain_write_jpg(const char *path,
                       const uint8_t *pixels,
                       uint32_t width,
                       uint32_t height,
                       int comp,
                       int quality)
{
    if (!path || !pixels) return -1;
    if (comp < 1 || comp > 4) comp = 1;
    if (comp == 2) comp = 1;  /* grey+alpha -> grayscale */
    if (quality < 1) quality = 80;
    if (quality > 100) quality = 100;

    /* JPEG has no alpha; RGBA must be converted to RGB */
    if (comp == 4) {
        size_t n = (size_t)width * (size_t)height;
        uint8_t *rgb = (uint8_t *)malloc(n * 3);
        if (!rgb) return -1;
        for (size_t i = 0; i < n; i++) {
            rgb[i * 3 + 0] = pixels[i * 4 + 0];
            rgb[i * 3 + 1] = pixels[i * 4 + 1];
            rgb[i * 3 + 2] = pixels[i * 4 + 2];
        }
        int ok = stbi_write_jpg(path, (int)width, (int)height, 3, rgb, quality) ? 0 : -1;
        free(rgb);
        return ok;
    }
    return stbi_write_jpg(path, (int)width, (int)height, comp, pixels, quality) ? 0 : -1;
}

int bitgrain_write_png(const char *path,
                       const uint8_t *pixels,
                       uint32_t width,
                       uint32_t height,
                       int comp)
{
    if (!path || !pixels) return -1;
    if (comp != 1 && comp != 3 && comp != 4) comp = 1;
    return stbi_write_png(path, (int)width, (int)height, comp, pixels, 0) ? 0 : -1;
}

#include <stdio.h>

int bitgrain_write_pgm(const char *path,
                       const uint8_t *pixels,
                       uint32_t width,
                       uint32_t height)
{
    if (!path || !pixels) return -1;
    FILE *f = fopen(path, "wb");
    if (!f) return -1;
    fprintf(f, "P5\n%u %u\n255\n", width, height);
    size_t n = (size_t)width * (size_t)height;
    int ok = (fwrite(pixels, 1, n, f) == n);
    fclose(f);
    return ok ? 0 : -1;
}
