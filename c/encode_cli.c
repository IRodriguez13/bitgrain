/* SPDX-License-Identifier: GPL-3.0-or-later */
#include "encode_cli.h"
#include "encoder.h"
#include "image_loader.h"
#include "config.h"
#include "bg_utils.h"
#include "path_utils.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/* Load image from path or stdin ("-"). Returns pixels and fills channels. */
static uint8_t *load_image(const char *path, uint32_t *w, uint32_t *h, uint32_t *channels,
                            const uint8_t *stdin_buf, size_t stdin_len)
{
    uint8_t *pixels;

    if (stdin_buf) {
        pixels = bitgrain_load_rgb_mem(stdin_buf, stdin_len, w, h);
        if (pixels) { *channels = 3; return pixels; }
        pixels = bitgrain_load_rgba_mem(stdin_buf, stdin_len, w, h);
        if (pixels) { *channels = 4; return pixels; }
        pixels = bitgrain_load_grayscale_mem(stdin_buf, stdin_len, w, h);
        if (pixels) { *channels = 1; return pixels; }
        return NULL;
    }

    /* Prefer true RGB for formats without alpha so .bg uses v4 (matches stbi native 3ch / bench). */
    pixels = bitgrain_load_rgb(path, w, h);
    if (pixels) { *channels = 3; return pixels; }
    pixels = bitgrain_load_rgba(path, w, h);
    if (pixels) { *channels = 4; return pixels; }
    pixels = bitgrain_load_grayscale(path, w, h);
    if (pixels) { *channels = 1; return pixels; }
    return NULL;
}

int encode_cli_run(const cli_ctx_t *ctx)
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

    int enc_failed = 0;
    for (size_t idx = 0; idx < ctx->expanded.n; idx++) {
        const char *cur_in = ctx->expanded.paths[idx];
        char *cur_out_owned = NULL;
        const char *cur_out;

        if (ctx->multi) {
            const char *base = strrchr(cur_in, '/');
            base = base ? base + 1 : cur_in;
            size_t base_len = strlen(base);
            size_t stem_len = base_len;
            if (is_image_extension(base)) {
                const char *dot = strrchr(base, '.');
                stem_len = dot ? (size_t)(dot - base) : base_len;
            }
            size_t dlen = strlen(ctx->output_dir);
            cur_out_owned = (char *)malloc(dlen + stem_len + 6);
            if (!cur_out_owned) {
                fprintf(stderr, "Error: out of memory.\n");
                enc_failed = 1; break;
            }
            snprintf(cur_out_owned, dlen + stem_len + 6, "%s/%.*s.bg",
                     ctx->output_dir, (int)stem_len, base);
            cur_out = cur_out_owned;
        } else {
            cur_out = ctx->output_path;
        }

        uint32_t width = 0, height = 0, channels = 0;
        const uint8_t *sbuf = (strcmp(cur_in, "-") == 0) ? stdin_buf : NULL;
        size_t slen = (strcmp(cur_in, "-") == 0) ? stdin_len : 0;
        uint8_t *pixels = load_image(cur_in, &width, &height, &channels, sbuf, slen);

        if (!pixels) {
            fprintf(stderr, "Error: could not load '%s'.\n", cur_in);
            free(cur_out_owned);
            enc_failed = 1;
            if (!ctx->multi) break;
            continue;
        }
        if (check_image_size(width, height, channels) != 0) {
            fprintf(stderr, "Error: image too large '%s' (max %u×%u).\n",
                    cur_in, BITGRAIN_MAX_DIM, BITGRAIN_MAX_DIM);
            bitgrain_image_free(pixels);
            free(cur_out_owned);
            enc_failed = 1;
            if (!ctx->multi) break;
            continue;
        }

        uint64_t raw_bytes = (uint64_t)width * height * channels;
        uint64_t out_cap = raw_bytes * 2 + BITGRAIN_OUT_BUF_MARGIN;
        if (out_cap > BITGRAIN_MAX_BG_FILE) out_cap = BITGRAIN_MAX_BG_FILE;
        uint8_t *out_buf = (uint8_t *)malloc((size_t)out_cap);
        if (!out_buf) {
            bitgrain_image_free(pixels);
            free(cur_out_owned);
            enc_failed = 1;
            if (!ctx->multi) break;
            continue;
        }

        int32_t out_len = 0;
        int ret;
        if (channels == 4)
            ret = bitgrain_encode_rgba(pixels, width, height, out_buf, (int32_t)out_cap, &out_len, (uint8_t)ctx->quality);
        else if (channels == 3)
            ret = bitgrain_encode_rgb(pixels, width, height, out_buf, (int32_t)out_cap, &out_len, (uint8_t)ctx->quality);
        else
            ret = bitgrain_encode_grayscale(pixels, width, height, out_buf, (int32_t)out_cap, &out_len, (uint8_t)ctx->quality);
        bitgrain_image_free(pixels);

        if (ret != 0) {
            fprintf(stderr, "Error: encoder failed '%s'.\n", cur_in);
            free(out_buf);
            free(cur_out_owned);
            enc_failed = 1;
            if (!ctx->multi) break;
            continue;
        }

        /* Write to stdout or file */
        if (ctx->use_stdout || strcmp(cur_out, "-") == 0) {
#ifdef _WIN32
            _setmode(_fileno(stdout), _O_BINARY);
#endif
            if (fwrite(out_buf, 1, (size_t)out_len, stdout) != (size_t)out_len) {
                fprintf(stderr, "Error: write to stdout failed.\n");
                free(out_buf);
                free(cur_out_owned);
                enc_failed = 1;
                break;
            }
            fflush(stdout);
        } else {
            FILE *out = fopen(cur_out, "wb");
            if (!out) {
                fprintf(stderr, "Error: could not create '%s'.\n", cur_out);
                free(out_buf);
                free(cur_out_owned);
                enc_failed = 1;
                if (!ctx->multi) break;
                continue;
            }
            if (fwrite(out_buf, 1, (size_t)out_len, out) != (size_t)out_len) {
                fprintf(stderr, "Error writing '%s'.\n", cur_out);
                fclose(out);
                free(out_buf);
                free(cur_out_owned);
                enc_failed = 1;
                if (!ctx->multi) break;
                continue;
            }
            fclose(out);
            fprintf(stderr, "%s -> %s  (%u×%u, %d bytes)\n",
                    cur_in, cur_out, width, height, (int)out_len);
        }

        free(out_buf);
        free(cur_out_owned);
    }

    free(stdin_buf);
    return enc_failed ? 1 : 0;
}
