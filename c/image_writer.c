#include "image_writer.h"

#define STB_IMAGE_WRITE_IMPLEMENTATION
#define STB_IMAGE_WRITE_STATIC
#include "stb_image_write.h"

int bitgrain_write_jpg(const char *path,
                       const uint8_t *pixels,
                       uint32_t width,
                       uint32_t height,
                       int quality)
{
    if (!path || !pixels) return -1;
    if (quality < 1) quality = 80;
    if (quality > 100) quality = 100;

    return stbi_write_jpg(path, (int)width, (int)height, 1, pixels, quality) ? 0 : -1;
}
