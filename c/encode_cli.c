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

int encode_cli_run(const cli_ctx_t *ctx)
{
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
                enc_failed = 1;
                break;
            }
            snprintf(cur_out_owned, dlen + stem_len + 6, "%s/%.*s.bg", ctx->output_dir, (int)stem_len, base);
            cur_out = cur_out_owned;
        } else {
            cur_out = ctx->output_path;
        }

        uint32_t width, height, channels;
        uint8_t *pixels = bitgrain_load_rgba(cur_in, &width, &height);
        if (pixels) channels = 4u;
        else {
            pixels = bitgrain_load_rgb(cur_in, &width, &height);
            if (pixels) channels = 3u;
            else {
                pixels = bitgrain_load_grayscale(cur_in, &width, &height);
                if (pixels) channels = 1u;
            }
        }
        if (!pixels) {
            fprintf(stderr, "Error: could not load '%s'.\n", cur_in);
            free(cur_out_owned);
            enc_failed = 1;
            if (!ctx->multi) break;
            continue;
        }
        if (check_image_size(width, height, channels) != 0) {
            fprintf(stderr, "Error: image too large '%s' (max %u×%u).\n", cur_in, BITGRAIN_MAX_DIM, BITGRAIN_MAX_DIM);
            bitgrain_image_free(pixels);
            free(cur_out_owned);
            enc_failed = 1;
            if (!ctx->multi) break;
            continue;
        }

        uint64_t raw_bytes = (uint64_t)width * height * channels;
        uint64_t out_cap = raw_bytes * 2 + BITGRAIN_OUT_BUF_MARGIN;
        if (out_cap > BITGRAIN_MAX_BG_FILE) out_cap = BITGRAIN_MAX_BG_FILE;
        size_t out_buf_size = (size_t)out_cap;
        uint8_t *out_buf = (uint8_t *)malloc(out_buf_size);
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
            ret = bitgrain_encode_rgba(pixels, width, height, out_buf, (int32_t)out_buf_size, &out_len, (uint8_t)ctx->quality);
        else if (channels == 3)
            ret = bitgrain_encode_rgb(pixels, width, height, out_buf, (int32_t)out_buf_size, &out_len, (uint8_t)ctx->quality);
        else
            ret = bitgrain_encode_grayscale(pixels, width, height, out_buf, (int32_t)out_buf_size, &out_len, (uint8_t)ctx->quality);
        bitgrain_image_free(pixels);
        if (ret != 0) {
            fprintf(stderr, "Error: encoder failed '%s'.\n", cur_in);
            free(out_buf);
            free(cur_out_owned);
            enc_failed = 1;
            if (!ctx->multi) break;
            continue;
        }

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
        free(out_buf);
        printf("%s -> %s  (%u×%u, %d bytes)\n", cur_in, cur_out, width, height, (int)out_len);
        free(cur_out_owned);
    }
    return enc_failed ? 1 : 0;
}
