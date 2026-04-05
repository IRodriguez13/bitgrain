/* SPDX-License-Identifier: GPL-3.0-or-later */
#include "decode_cli.h"
#include "encoder.h"
#include "image_loader.h"
#include "config.h"
#include "image_writer.h"
#include "webp_io.h"
#include "bg_utils.h"
#include "path_utils.h"
#include "platform.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/* Write decoded pixels to file or stdout.
   For stdout: emit raw PPM/PGM so it's pipe-friendly. */
static int write_output(const char *cur_out, int use_stdout,
                        const uint8_t *pixels,
                        uint32_t width, uint32_t height, uint32_t channels,
                        int jpeg_quality)
{
    if (use_stdout || strcmp(cur_out, "-") == 0) {
#ifdef _WIN32
        _setmode(_fileno(stdout), _O_BINARY);
#endif
        /* Emit raw PPM (RGB) or PGM (gray) to stdout */
        if (channels == 1) {
            fprintf(stdout, "P5\n%u %u\n255\n", width, height);
        } else {
            fprintf(stdout, "P6\n%u %u\n255\n", width, height);
        }
        size_t npix = (size_t)width * height * (channels == 1 ? 1 : 3);
        /* For RGBA stdout: drop alpha, emit RGB */
        if (channels == 4) {
            for (size_t i = 0; i < (size_t)width * height; i++) {
                fwrite(pixels + i * 4, 1, 3, stdout);
            }
        } else {
            fwrite(pixels, 1, npix, stdout);
        }
        fflush(stdout);
        return 1;
    }

    size_t len = strlen(cur_out);
    if (channels <= 3 && len >= 4 && (strcmp(cur_out + len - 4, ".jpg") == 0 ||
            (len >= 5 && strcmp(cur_out + len - 5, ".jpeg") == 0)))
        return bitgrain_write_jpg(cur_out, pixels, width, height, (int)channels, jpeg_quality) == 0;
    if (len >= 4 && strcmp(cur_out + len - 4, ".png") == 0)
        return bitgrain_write_png(cur_out, pixels, width, height, (int)channels) == 0;
    if (len >= 4 && strcmp(cur_out + len - 4, ".bmp") == 0)
        return bitgrain_write_bmp(cur_out, pixels, width, height, (int)channels) == 0;
    if (len >= 4 && strcmp(cur_out + len - 4, ".tga") == 0)
        return bitgrain_write_tga(cur_out, pixels, width, height, (int)channels) == 0;
    if (len >= 5 && strcmp(cur_out + len - 5, ".webp") == 0)
        return bitgrain_write_webp(cur_out, pixels, width, height, (int)channels, jpeg_quality) == 0;
    if (channels == 1)
        return bitgrain_write_pgm(cur_out, pixels, width, height) == 0;
    return 0;
}

/* Read .bg data from path or stdin buffer. Returns malloc'd buffer, sets *fsize. */
static uint8_t *read_bg(const char *cur_in, int use_stdin,
                        const uint8_t *stdin_buf, size_t stdin_len,
                        long *fsize)
{
    if (use_stdin || strcmp(cur_in, "-") == 0) {
        *fsize = (long)stdin_len;
        uint8_t *buf = (uint8_t *)malloc(stdin_len);
        if (!buf) return NULL;
        memcpy(buf, stdin_buf, stdin_len);
        return buf;
    }

    FILE *f = fopen(cur_in, "rb");
    if (!f) return NULL;
    if (fseek(f, 0, SEEK_END) != 0) { fclose(f); return NULL; }
    *fsize = ftell(f);
    if (*fsize <= 0 || *fsize > (long)BITGRAIN_MAX_BG_FILE) { fclose(f); return NULL; }
    rewind(f);
    uint8_t *buf = (uint8_t *)malloc((size_t)*fsize);
    if (!buf) { fclose(f); return NULL; }
    if (fread(buf, 1, (size_t)*fsize, f) != (size_t)*fsize) { fclose(f); free(buf); return NULL; }
    fclose(f);
    return buf;
}

int decode_cli_run(const cli_ctx_t *ctx)
{
    /* Read stdin once if needed */
    uint8_t *stdin_buf = NULL;
    size_t   stdin_len = 0;
    if (ctx->use_stdin) {
#ifdef _WIN32
        _setmode(_fileno(stdin), _O_BINARY);
#endif
        stdin_buf = bitgrain_read_stream(stdin, &stdin_len);
        if (!stdin_buf) {
            fprintf(stderr, "Error: could not read from stdin.\n");
            return 1;
        }
    }

    int dec_failed = 0;
    for (size_t idx = 0; idx < ctx->expanded.n; idx++) {
        const char *cur_in = ctx->expanded.paths[idx];
        char *cur_out_owned = NULL;
        const char *cur_out;

        if (ctx->multi) {
            const char *base = strrchr(cur_in, '/');
            base = base ? base + 1 : cur_in;
            size_t base_len = strlen(base);
            size_t dlen = strlen(ctx->output_dir);
            size_t stem_len = (base_len >= 3 &&
                platform_strcasecmp(base + base_len - 3, ".bg") == 0) ? base_len - 3 : base_len;
            cur_out_owned = (char *)malloc(dlen + stem_len + 6);
            if (!cur_out_owned) { fprintf(stderr, "Error: out of memory.\n"); dec_failed = 1; break; }
            snprintf(cur_out_owned, dlen + stem_len + 6, "%s/%.*s.jpg",
                     ctx->output_dir, (int)stem_len, base);
            cur_out = cur_out_owned;
            if (!ctx->overwrite) {
                FILE *ex = fopen(cur_out, "rb");
                if (ex) {
                    fclose(ex);
                    char *alt = avoid_overwrite_path(cur_out);
                    if (alt) { free(cur_out_owned); cur_out_owned = alt; cur_out = cur_out_owned; }
                }
            }
        } else {
            cur_out = ctx->output_path;
        }

        long fsize = 0;
        int is_stdin = (strcmp(cur_in, "-") == 0);
        uint8_t *bg_buf = read_bg(cur_in, is_stdin, stdin_buf, stdin_len, &fsize);
        if (!bg_buf) {
            fprintf(stderr, "Error: could not read '%s'.\n", cur_in);
            free(cur_out_owned);
            dec_failed = 1;
            if (!ctx->multi) break;
            continue;
        }

        uint32_t width, height, channels;
        if (fsize < 11 || parse_bg_header(bg_buf, &width, &height, &channels) != 0) {
            fprintf(stderr, "Error: '%s' is not a valid .bg or is corrupt.\n", cur_in);
            free(bg_buf);
            free(cur_out_owned);
            dec_failed = 1;
            if (!ctx->multi) break;
            continue;
        }
        if (check_image_size(width, height, channels) != 0) {
            fprintf(stderr, "Error: .bg image dimensions too large '%s'.\n", cur_in);
            free(bg_buf);
            free(cur_out_owned);
            dec_failed = 1;
            if (!ctx->multi) break;
            continue;
        }

        size_t pixel_bytes = (size_t)width * height * channels;
        uint8_t *pixels = (uint8_t *)malloc(pixel_bytes);
        if (!pixels) {
            free(bg_buf);
            free(cur_out_owned);
            dec_failed = 1;
            if (!ctx->multi) break;
            continue;
        }

        int ret = bitgrain_decode(bg_buf, (int32_t)fsize, pixels, (uint32_t)pixel_bytes,
                                  &width, &height, &channels);
        free(bg_buf);
        if (ret != 0) {
            fprintf(stderr, "Error: '%s' is not a valid .bg or is corrupt.\n", cur_in);
            free(pixels);
            free(cur_out_owned);
            dec_failed = 1;
            if (!ctx->multi) break;
            continue;
        }

        int out_is_stdout = ctx->use_stdout || strcmp(cur_out, "-") == 0;
        if (!write_output(cur_out, out_is_stdout, pixels, width, height, channels,
                          ctx->jpeg_out_quality)) {
            fprintf(stderr, "Error: could not write '%s'.\n", cur_out);
            free(pixels);
            free(cur_out_owned);
            dec_failed = 1;
            if (!ctx->multi) break;
            continue;
        }

        if (!out_is_stdout)
            fprintf(stderr, "%s -> %s  (%u×%u, %u ch)\n", cur_in, cur_out, width, height, channels);

        free(pixels);
        free(cur_out_owned);
    }

    free(stdin_buf);
    return dec_failed ? 1 : 0;
}
