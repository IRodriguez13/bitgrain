/* SPDX-License-Identifier: GPL-3.0-or-later */
#include "icc_io.h"
#include <stdlib.h>
#include <string.h>

#ifdef BITGRAIN_USE_PNG_ICC
#include <png.h>
#include <stdio.h>

uint8_t *bitgrain_load_icc_from_png(const char *path, uint32_t *out_len)
{
    FILE *f;
    png_structp png;
    png_infop info;
    png_bytep profile = NULL;
    png_uint_32 proflen = 0;
    uint8_t *copy = NULL;

    if (!path || !out_len) return NULL;
    *out_len = 0;

    f = fopen(path, "rb");
    if (!f) return NULL;

    png = png_create_read_struct(PNG_LIBPNG_VER_STRING, NULL, NULL, NULL);
    if (!png) { fclose(f); return NULL; }

    info = png_create_info_struct(png);
    if (!info) { png_destroy_read_struct(&png, NULL, NULL); fclose(f); return NULL; }

    if (setjmp(png_jmpbuf(png))) {
        png_destroy_read_struct(&png, &info, NULL);
        fclose(f);
        return NULL;
    }

    png_init_io(png, f);
    png_read_info(png, info);

    if (png_get_valid(png, info, PNG_INFO_iCCP)) {
        png_charp name;
        int compression_type;
        if (png_get_iCCP(png, info, &name, &compression_type, &profile, &proflen)) {
            if (profile && proflen > 0) {
                copy = (uint8_t *)malloc(proflen);
                if (copy) {
                    memcpy(copy, profile, proflen);
                    *out_len = (uint32_t)proflen;
                }
            }
        }
    }

    png_destroy_read_struct(&png, &info, NULL);
    fclose(f);
    return copy;
}

int bitgrain_write_png_with_icc(const char *path,
                                const uint8_t *pixels,
                                uint32_t width,
                                uint32_t height,
                                int comp,
                                const uint8_t *icc,
                                uint32_t icc_len)
{
    FILE *f;
    png_structp png;
    png_infop info;
    int color_type, bit_depth = 8;
    png_bytep *row_pointers = NULL;
    int y, ok = -1;

    if (!path || !pixels) return -1;
    if (comp != 1 && comp != 3 && comp != 4) comp = 3;

    f = fopen(path, "wb");
    if (!f) return -1;

    png = png_create_write_struct(PNG_LIBPNG_VER_STRING, NULL, NULL, NULL);
    if (!png) { fclose(f); return -1; }

    info = png_create_info_struct(png);
    if (!info) { png_destroy_write_struct(&png, NULL); fclose(f); return -1; }

    if (setjmp(png_jmpbuf(png))) {
        png_destroy_write_struct(&png, &info);
        free(row_pointers);
        fclose(f);
        return -1;
    }

    png_init_io(png, f);

    if (comp == 1) color_type = PNG_COLOR_TYPE_GRAY;
    else if (comp == 3) color_type = PNG_COLOR_TYPE_RGB;
    else color_type = PNG_COLOR_TYPE_RGBA;

    png_set_IHDR(png, info, (png_uint_32)width, (png_uint_32)height, bit_depth,
                 color_type, PNG_INTERLACE_NONE, PNG_COMPRESSION_TYPE_DEFAULT,
                 PNG_FILTER_TYPE_DEFAULT);

    if (icc && icc_len > 0) {
        png_set_iCCP(png, info, "ICC Profile", 0, (png_const_bytep)icc, (png_uint_32)icc_len);
    }

    png_write_info(png, info);

    row_pointers = (png_bytep *)malloc(height * sizeof(png_bytep));
    if (!row_pointers) goto cleanup;

    for (y = 0; y < (int)height; y++) {
        row_pointers[y] = (png_bytep)(pixels + (size_t)y * width * comp);
    }
    png_write_image(png, row_pointers);
    png_write_end(png, info);
    ok = 0;

cleanup:
    free(row_pointers);
    png_destroy_write_struct(&png, &info);
    fclose(f);
    return ok;
}

#else

uint8_t *bitgrain_load_icc_from_png(const char *path, uint32_t *out_len)
{
    (void)path;
    if (out_len) *out_len = 0;
    return NULL;
}

int bitgrain_write_png_with_icc(const char *path,
                                const uint8_t *pixels,
                                uint32_t width,
                                uint32_t height,
                                int comp,
                                const uint8_t *icc,
                                uint32_t icc_len)
{
    (void)path;
    (void)pixels;
    (void)width;
    (void)height;
    (void)comp;
    (void)icc;
    (void)icc_len;
    return -1; /* Not implemented without libpng */
}

#endif
