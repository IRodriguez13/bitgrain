/* SPDX-License-Identifier: GPL-3.0-or-later  Copyright (C) 2026 Iván E. Rodriguez */

#define _POSIX_C_SOURCE 200809L

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include <unistd.h>

#include "encoder.h"
#include "image_loader.h"
#include "image_writer.h"

#define OUT_BUF_SIZE (1024 * 1024 * 4)
#define MAX_PIXELS   (4096 * 4096)
#define MAX_PIXELS_RGB (MAX_PIXELS * 3)

static void usage(const char *prog)
{
    fprintf(stderr,
            "bitgrain – compress and decompress images (encode ↔ decode)\n\n"
            "Compress (image → .bg):\n"
            "  %s -i <image> -o <output.bg>\n"
            "  %s <image>                     → <image>.bg\n\n"
            "Decompress (.bg → visible image):\n"
            "  %s -d -i <file.bg> -o <image.jpg>\n"
            "  %s -d <file.bg>                → <file>.jpg\n\n"
            "Round-trip (compress + decompress in one step, no .bg file):\n"
            "  %s -cd -i <image> -o <reconstructed.jpg>\n\n"
            "Options:\n"
            "  -i <file>   input (image or .bg depending on mode)\n"
            "  -o <file>   output\n"
            "  -d          decompress (.bg → JPG or PGM per -o)\n"
            "  -cd         compress then decompress in memory, write image (no .bg)\n"
            "  -y          overwrite output\n"
            "  -h          this help\n",
            prog, prog, prog, prog, prog);
}

/* Returns extension (including the dot) or "" if none. */
static const char *get_ext(const char *path)
{
    const char *dot = strrchr(path, '.');
    if (!dot || dot == path) return "";
    return dot;
}

/* Default output: encode → .bg, decode/round-trip → .jpg */
static int default_output_path(const char *input, char *out_buf, size_t buf_size, int decode_mode, int round_trip)
{
    const char *ext = get_ext(input);
    size_t base_len = ext ? (size_t)(ext - input) : strlen(input);
    if (decode_mode || round_trip) {
        if (base_len + 5 >= buf_size) return -1;
        memcpy(out_buf, input, base_len);
        memcpy(out_buf + base_len, ".jpg", 5);
    } else {
        if (base_len + 4 >= buf_size) return -1;
        memcpy(out_buf, input, base_len);
        memcpy(out_buf + base_len, ".bg", 4);
    }
    return 0;
}

static int write_pgm(const char *path, const uint8_t *pixels, uint32_t width, uint32_t height)
{
    FILE *f = fopen(path, "wb");

    if (!f) return -1;

    fprintf(f, "P5\n%u %u\n255\n", width, height);
    size_t n = (size_t)width * (size_t)height;

    if (fwrite(pixels, 1, n, f) != n) 
    {
        fclose(f);
        return -1;
    }
    fclose(f);
    return 0;
}

int main(int argc, char **argv)
{
    const char *input_path  = NULL;
    const char *output_path = NULL;
    char *output_path_owned = NULL;
    int overwrite = 0;
    int decode_mode = 0;
    int round_trip = 0;
    int opt;

    while ((opt = getopt(argc, argv, "i:o:cdyh")) != -1) {
        switch (opt) {
        case 'i':
            input_path = optarg;
            break;
        case 'o':
            output_path = optarg;
            break;
        case 'c':
            round_trip = 1;
            break;
        case 'd':
            if (!round_trip) decode_mode = 1;
            break;
        case 'y':
            overwrite = 1;
            break;
        case 'h':
            usage(argv[0]);
            return 0;
        default:
            usage(argv[0]);
            return 1;
        }
    }

    if (!input_path && optind < argc) 
    {
        input_path = argv[optind];
    }

    if (!input_path) 
    {
        fprintf(stderr, "Error: missing input file.\n");
        usage(argv[0]);
        return 1;
    }

    if (!output_path) {
        char def[1024];
        if (default_output_path(input_path, def, sizeof(def), decode_mode, round_trip) != 0) {
            fprintf(stderr, "Error: input path too long.\n");
            return 1;
        }
        output_path_owned = strdup(def);
        if (!output_path_owned) {
            fprintf(stderr, "Error: out of memory.\n");
            return 1;
        }
        output_path = output_path_owned;
    }

    if (!overwrite) {
        FILE *exists = fopen(output_path, "rb");
        if (exists) {
            fclose(exists);
            fprintf(stderr, "Error: '%s' already exists. Use -y to overwrite.\n", output_path);
            free(output_path_owned);
            return 1;
        }
    }

    if (round_trip) {
        /* Load → encode to memory → decode → write image (no .bg file) */
        uint32_t width, height, channels;
        uint8_t *pixels = bitgrain_load_rgb(input_path, &width, &height);
        int use_rgb = (pixels != NULL);
        if (!pixels) {
            pixels = bitgrain_load_grayscale(input_path, &width, &height);
        }
        if (!pixels) {
            fprintf(stderr, "Error: could not load '%s'.\n", input_path);
            free(output_path_owned);
            return 1;
        }

        uint8_t *out_buf = (uint8_t *)malloc(OUT_BUF_SIZE);
        if (!out_buf) {
            bitgrain_image_free(pixels);
            free(output_path_owned);
            return 1;
        }

        int32_t out_len = 0;
        int ret;
        if (use_rgb)
            ret = bitgrain_encode_rgb(pixels, width, height, out_buf, OUT_BUF_SIZE, &out_len);
        else
            ret = bitgrain_encode_grayscale(pixels, width, height, out_buf, OUT_BUF_SIZE, &out_len);
        bitgrain_image_free(pixels);
        if (ret != 0) {
            fprintf(stderr, "Error: encode failed.\n");
            free(out_buf);
            free(output_path_owned);
            return 1;
        }

        uint8_t *decoded = (uint8_t *)malloc(MAX_PIXELS_RGB);
        if (!decoded) {
            free(out_buf);
            free(output_path_owned);
            return 1;
        }
        ret = bitgrain_decode(out_buf, out_len, decoded, MAX_PIXELS_RGB, &width, &height, &channels);
        free(out_buf);
        if (ret != 0) {
            fprintf(stderr, "Error: decode failed.\n");
            free(decoded);
            free(output_path_owned);
            return 1;
        }

        size_t out_len_str = strlen(output_path);
        int written = 0;
        if (out_len_str >= 4 && (strcmp(output_path + out_len_str - 4, ".jpg") == 0 ||
                                 (out_len_str >= 5 && strcmp(output_path + out_len_str - 5, ".jpeg") == 0)))
            written = (bitgrain_write_jpg(output_path, decoded, width, height, (int)channels, 90) == 0);
        if (!written)
            written = (channels == 1 && write_pgm(output_path, decoded, width, height) == 0);
        free(decoded);
        if (!written) {
            fprintf(stderr, "Error: could not write '%s' (use .jpg for color).\n", output_path);
            free(output_path_owned);
            return 1;
        }
        printf("%s -> %s  (%u×%u, round-trip)\n", input_path, output_path, width, height);
        free(output_path_owned);
        return 0;
    }

    if (decode_mode) 
    {
        /* Decompress .bg → image (JPG/PGM) */
        FILE *f = fopen(input_path, "rb");
        if (!f) {
            fprintf(stderr, "Error: could not open '%s'.\n", input_path);
            free(output_path_owned);
            return 1;
        }
        if (fseek(f, 0, SEEK_END) != 0) {
            fclose(f);
            free(output_path_owned);
            return 1;
        }
        long fsize = ftell(f);
        if (fsize <= 0 || fsize > (long)OUT_BUF_SIZE) {
            fclose(f);
            free(output_path_owned);
            fprintf(stderr, "Error: .bg file invalid or too large.\n");
            return 1;
        }
        rewind(f);
        uint8_t *bg_buf = (uint8_t *)malloc((size_t)fsize);
        if (!bg_buf) {
            fclose(f);
            free(output_path_owned);
            return 1;
        }
        if (fread(bg_buf, 1, (size_t)fsize, f) != (size_t)fsize) {
            fclose(f);
            free(bg_buf);
            free(output_path_owned);
            return 1;
        }
        fclose(f);

        uint8_t *pixels = (uint8_t *)malloc(MAX_PIXELS_RGB);
        if (!pixels) {
            free(bg_buf);
            free(output_path_owned);
            return 1;
        }
        uint32_t width, height, channels;
        int ret = bitgrain_decode(bg_buf, (int32_t)fsize, pixels, MAX_PIXELS_RGB,
                                  &width, &height, &channels);
        free(bg_buf);
        if (ret != 0) {
            fprintf(stderr, "Error: '%s' is not a valid .bg or is corrupt.\n", input_path);
            free(pixels);
            free(output_path_owned);
            return 1;
        }
        {
            size_t out_len = strlen(output_path);
            int written = 0;
            if (out_len >= 4 && (strcmp(output_path + out_len - 4, ".jpg") == 0 ||
                                 (out_len >= 5 && strcmp(output_path + out_len - 5, ".jpeg") == 0)))
                written = (bitgrain_write_jpg(output_path, pixels, width, height, (int)channels, 90) == 0);
            if (!written)
                written = (channels == 1 && write_pgm(output_path, pixels, width, height) == 0);
            if (!written) {
                fprintf(stderr, "Error: could not write '%s'.\n", output_path);
                free(pixels);
                free(output_path_owned);
                return 1;
            }
        }
        free(pixels);
        printf("%s -> %s  (%u×%u, %u channel(s))\n", input_path, output_path, width, height, channels);
        free(output_path_owned);
        return 0;
    }

    /* Compress image → .bg (color if image has color, else grayscale) */
    uint32_t width, height;
    uint8_t *pixels = bitgrain_load_rgb(input_path, &width, &height);
    int use_rgb = (pixels != NULL);
    if (!pixels) {
        pixels = bitgrain_load_grayscale(input_path, &width, &height);
    }
    if (!pixels) {
        fprintf(stderr, "Error: could not load '%s' (does file exist and is it a valid image?).\n", input_path);
        free(output_path_owned);
        return 1;
    }

    uint8_t *out_buf = (uint8_t *)malloc(OUT_BUF_SIZE);
    if (!out_buf) {
        bitgrain_image_free(pixels);
        free(output_path_owned);
        return 1;
    }

    int32_t out_len = 0;
    int ret;
    if (use_rgb)
        ret = bitgrain_encode_rgb(pixels, width, height, out_buf, OUT_BUF_SIZE, &out_len);
    else
        ret = bitgrain_encode_grayscale(pixels, width, height, out_buf, OUT_BUF_SIZE, &out_len);
    bitgrain_image_free(pixels);

    if (ret != 0) 
    {
        fprintf(stderr, "Error: encoder failed.\n");
        free(out_buf);
        free(output_path_owned);
        return 1;
    }

    FILE *out = fopen(output_path, "wb");
    
    if (!out) 
    {
        fprintf(stderr, "Error: could not create '%s'.\n", output_path);
        free(out_buf);
        free(output_path_owned);
        return 1;
    }

    if (fwrite(out_buf, 1, (size_t)out_len, out) != (size_t)out_len) 
    {
        fprintf(stderr, "Error writing output.\n");
        fclose(out);
        free(out_buf);
        free(output_path_owned);
        return 1;
    }

    fclose(out);
    free(out_buf);

    printf("%s -> %s  (%u×%u, %d bytes)\n",
           input_path, output_path, width, height, (int)out_len);
    free(output_path_owned);
    return 0;
}
