#include "image_loader.h"
#include <stdlib.h>

#define STB_IMAGE_IMPLEMENTATION
#define STB_IMAGE_STATIC
#include "stb_image.h"

uint8_t *bitgrain_load_grayscale(const char *path,
                                  uint32_t *out_width,
                                  uint32_t *out_height)
{
    int w, h, n;
    unsigned char *data = stbi_load(path, &w, &h, &n, 1); /* 1 = 1 canal (gris); RGB se convierte automático */
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

void bitgrain_image_free(void *pixels)
{
    stbi_image_free(pixels);
}
