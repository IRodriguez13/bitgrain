/* SPDX-License-Identifier: GPL-3.0-or-later  Copyright (C) 2026 Iván E. Rodriguez */

#define _POSIX_C_SOURCE 200809L
#define BITGRAIN_VERSION "1.0.0"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include <unistd.h>

#include "encoder.h"
#include "image_loader.h"
#include "image_writer.h"

/* Dynamic allocation with caps to support very large images (100 MB, 1 GB, up to ~2 GB pixel buffer). */
#define MAX_DIM         65536
#define MAX_PIXEL_BYTES  (2ULL * 1024 * 1024 * 1024)
#define MAX_BG_FILE      (2ULL * 1024 * 1024 * 1024)
#define OUT_BUF_MARGIN   (1024 * 1024)

/* Parse .bg header (11 bytes): magic + version + width + height LE. Sets *channels (1 or 3). Returns 0 on success. */
static int parse_bg_header(const uint8_t *buf, uint32_t *width, uint32_t *height, uint32_t *channels)
{
    if (buf[0] != 'B' || buf[1] != 'G') return -1;
    uint8_t ver = buf[2];
    if (ver != 1 && ver != 2) return -1;
    *width  = (uint32_t)buf[3] | ((uint32_t)buf[4]<<8) | ((uint32_t)buf[5]<<16) | ((uint32_t)buf[6]<<24);
    *height = (uint32_t)buf[7] | ((uint32_t)buf[8]<<8) | ((uint32_t)buf[9]<<16) | ((uint32_t)buf[10]<<24);
    *channels = (ver == 1) ? 1u : 3u;
    return 0;
}

static int check_image_size(uint32_t width, uint32_t height, uint32_t channels)
{
    if (width == 0 || height == 0 || width > MAX_DIM || height > MAX_DIM) return -1;
    uint64_t bytes = (uint64_t)width * height * channels;
    if (bytes > MAX_PIXEL_BYTES) return -1;
    return 0;
}


static void usage(const char *prog)
{
    fprintf(stderr,
            "bitgrain " BITGRAIN_VERSION " – image compressor (JPEG-like .bg format)\n"
            "  encode: image → .bg   decode: .bg → image   round-trip: image → image (no .bg file)\n\n"
            "Usage:\n"
            "  %s -i <in> -o <out>       encode image to .bg\n"
            "  %s <image>               → <image>.bg\n"
            "  %s -d -i <file.bg> -o <out>   decode .bg to image (.jpg/.png/.pgm by -o)\n"
            "  %s -cd -i <image> -o <out>    round-trip: compress+decompress in memory\n\n"
            "Options:\n"
            "  -i <file>   input (image or .bg)\n"
            "  -o <file>   output (format by extension: .jpg .png .pgm)\n"
            "  -d          decode (.bg → image)\n"
            "  -cd         round-trip (no .bg file written)\n"
            "  -q <1-100>  .bg encode quality (default 85)\n"
            "  -Q <1-100>  output JPG quality when writing .jpg (default 85; smaller file)\n"
            "  -y          overwrite\n"
            "  -v          version\n"
            "  -h          help\n",
            prog, prog, prog, prog);
}

/* Returns extension (including the dot) or "" if none. */
static const char *get_ext(const char *path)
{
    const char *dot = strrchr(path, '.');
    if (!dot || dot == path) return "";
    return dot;
}

/* Default output: encode → .bg, decode/round-trip → .jpg.
 * If output would equal input (e.g. input is .jpg and we default to .jpg), use "base (1).ext" instead. */
static int default_output_path(const char *input, char *out_buf, size_t buf_size, int decode_mode, int round_trip)
{
    const char *ext = get_ext(input);
    size_t base_len = ext ? (size_t)(ext - input) : strlen(input);
    size_t ext_len = (decode_mode || round_trip) ? 5u : 4u;  /* including null */

    if (decode_mode || round_trip) {
        if (base_len + ext_len >= buf_size) return -1;
        memcpy(out_buf, input, base_len);
        memcpy(out_buf + base_len, ".jpg", ext_len);
        if (strcmp(out_buf, input) == 0) {
            if (base_len + 9 >= buf_size) return -1;  /* " (1).jpg" + null */
            memcpy(out_buf + base_len, " (1).jpg", 8);
            out_buf[base_len + 8] = '\0';
        }
    } else {
        if (base_len + ext_len >= buf_size) return -1;
        memcpy(out_buf, input, base_len);
        memcpy(out_buf + base_len, ".bg", ext_len);
    }
    return 0;
}

/* If path exists and we're not overwriting, return a new path "base (n).ext" that does not exist. Caller must free. */
static char *avoid_overwrite_path(const char *path)
{
    const char *ext = get_ext(path);
    size_t base_len = ext ? (size_t)(ext - path) : strlen(path);
    size_t slen = base_len + 32;
    char *out = (char *)malloc(slen);
    if (!out) return NULL;

    memcpy(out, path, base_len);
    out[base_len] = '\0';
    for (int n = 1; n <= 9999; n++) {
        int len = snprintf(out + base_len, slen - base_len, " (%d)%s", n, ext ? ext : "");
        if (len < 0 || (size_t)len >= slen - base_len) break;
        FILE *f = fopen(out, "rb");
        if (!f) return out;
        fclose(f);
    }
    free(out);
    return NULL;
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

    int quality = 85;
    int jpeg_out_quality = 85;
    while ((opt = getopt(argc, argv, "i:o:cdq:Q:yvh")) != -1) {
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
        case 'q':
            quality = atoi(optarg);
            if (quality < 1) quality = 1;
            if (quality > 100) quality = 100;
            break;
        case 'Q':
            jpeg_out_quality = atoi(optarg);
            if (jpeg_out_quality < 1) jpeg_out_quality = 1;
            if (jpeg_out_quality > 100) jpeg_out_quality = 100;
            break;
        case 'y':
            overwrite = 1;
            break;
        case 'v':
            printf("bitgrain %s\n", BITGRAIN_VERSION);
            printf("Author: Iván E. Rodriguez\n");
            printf("License: GPLv3\n");
            printf("Upstream: https://github.com/IRodriguez13/bitgrain\n");
            return 0;
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
            char *alt = avoid_overwrite_path(output_path);
            if (alt) {
                free(output_path_owned);
                output_path_owned = alt;
                output_path = output_path_owned;
            } else {
                fprintf(stderr, "Error: '%s' already exists. Use -y to overwrite.\n", output_path);
                free(output_path_owned);
                return 1;
            }
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
        channels = use_rgb ? 3u : 1u;
        if (check_image_size(width, height, channels) != 0) {
            fprintf(stderr, "Error: image too large (max %u×%u or %llu pixel bytes).\n",
                    MAX_DIM, MAX_DIM, (unsigned long long)MAX_PIXEL_BYTES);
            bitgrain_image_free(pixels);
            free(output_path_owned);
            return 1;
        }

        uint64_t raw_bytes = (uint64_t)width * height * channels;
        uint64_t out_cap = raw_bytes * 2 + OUT_BUF_MARGIN;
        if (out_cap > MAX_BG_FILE) out_cap = MAX_BG_FILE;
        size_t out_buf_size = (size_t)out_cap;
        uint8_t *out_buf = (uint8_t *)malloc(out_buf_size);
        if (!out_buf) {
            bitgrain_image_free(pixels);
            free(output_path_owned);
            return 1;
        }

        int32_t out_len = 0;
        int ret;
        if (use_rgb)
            ret = bitgrain_encode_rgb(pixels, width, height, out_buf, (int32_t)out_buf_size, &out_len, (uint8_t)quality);
        else
            ret = bitgrain_encode_grayscale(pixels, width, height, out_buf, (int32_t)out_buf_size, &out_len, (uint8_t)quality);
        bitgrain_image_free(pixels);
        if (ret != 0) {
            fprintf(stderr, "Error: encode failed (output buffer may be too small).\n");
            free(out_buf);
            free(output_path_owned);
            return 1;
        }

        size_t decoded_size = (size_t)raw_bytes;
        uint8_t *decoded = (uint8_t *)malloc(decoded_size);
        if (!decoded) {
            free(out_buf);
            free(output_path_owned);
            return 1;
        }
        ret = bitgrain_decode(out_buf, out_len, decoded, (int32_t)decoded_size, &width, &height, &channels);
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
            written = (bitgrain_write_jpg(output_path, decoded, width, height, (int)channels, jpeg_out_quality) == 0);
        if (!written && out_len_str >= 4 && strcmp(output_path + out_len_str - 4, ".png") == 0)
            written = (bitgrain_write_png(output_path, decoded, width, height, (int)channels) == 0);
        if (!written)
            written = (channels == 1 && write_pgm(output_path, decoded, width, height) == 0);
        free(decoded);
        if (!written) {
            fprintf(stderr, "Error: could not write '%s' (use .jpg, .png, or .pgm for grayscale).\n", output_path);
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
        if (fsize <= 0 || fsize > (long)MAX_BG_FILE) {
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

        uint32_t width, height, channels;
        if (fsize < 11 || parse_bg_header(bg_buf, &width, &height, &channels) != 0) {
            fprintf(stderr, "Error: '%s' is not a valid .bg or is corrupt.\n", input_path);
            free(bg_buf);
            free(output_path_owned);
            return 1;
        }
        if (check_image_size(width, height, channels) != 0) {
            fprintf(stderr, "Error: .bg image dimensions too large.\n");
            free(bg_buf);
            free(output_path_owned);
            return 1;
        }
        size_t pixel_bytes = (size_t)width * height * channels;
        uint8_t *pixels = (uint8_t *)malloc(pixel_bytes);
        if (!pixels) {
            free(bg_buf);
            free(output_path_owned);
            return 1;
        }
        int ret = bitgrain_decode(bg_buf, (int32_t)fsize, pixels, (int32_t)pixel_bytes,
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
                written = (bitgrain_write_jpg(output_path, pixels, width, height, (int)channels, jpeg_out_quality) == 0);
            if (!written && out_len >= 4 && strcmp(output_path + out_len - 4, ".png") == 0)
                written = (bitgrain_write_png(output_path, pixels, width, height, (int)channels) == 0);
            if (!written)
                written = (channels == 1 && write_pgm(output_path, pixels, width, height) == 0);
            if (!written) {
                fprintf(stderr, "Error: could not write '%s' (use .jpg, .png, or .pgm).\n", output_path);
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
    uint32_t channels = use_rgb ? 3u : 1u;
    if (check_image_size(width, height, channels) != 0) {
        fprintf(stderr, "Error: image too large (max %u×%u or %llu pixel bytes).\n",
                MAX_DIM, MAX_DIM, (unsigned long long)MAX_PIXEL_BYTES);
        bitgrain_image_free(pixels);
        free(output_path_owned);
        return 1;
    }

    uint64_t raw_bytes = (uint64_t)width * height * channels;
    uint64_t out_cap = raw_bytes * 2 + OUT_BUF_MARGIN;
    if (out_cap > MAX_BG_FILE) out_cap = MAX_BG_FILE;
    size_t out_buf_size = (size_t)out_cap;
    uint8_t *out_buf = (uint8_t *)malloc(out_buf_size);
    if (!out_buf) {
        bitgrain_image_free(pixels);
        free(output_path_owned);
        return 1;
    }

    int32_t out_len = 0;
    int ret;
    if (use_rgb)
        ret = bitgrain_encode_rgb(pixels, width, height, out_buf, (int32_t)out_buf_size, &out_len, (uint8_t)quality);
    else
        ret = bitgrain_encode_grayscale(pixels, width, height, out_buf, (int32_t)out_buf_size, &out_len, (uint8_t)quality);
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
