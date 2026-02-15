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

static int write_output(const char *cur_out, const uint8_t *pixels,
                        uint32_t width, uint32_t height, uint32_t channels,
                        int jpeg_quality)
{
    size_t len = strlen(cur_out);
    if (channels <= 3 && len >= 4 && (strcmp(cur_out + len - 4, ".jpg") == 0 ||
            (len >= 5 && strcmp(cur_out + len - 5, ".jpeg") == 0)))
        return bitgrain_write_jpg(cur_out, pixels, width, height, (int)channels, jpeg_quality) == 0;
    if (len >= 4 && strcmp(cur_out + len - 4, ".png") == 0)
        return bitgrain_write_png(cur_out, pixels, width, height, (int)channels) == 0;
    if (len >= 5 && strcmp(cur_out + len - 5, ".webp") == 0)
        return bitgrain_write_webp(cur_out, pixels, width, height, (int)channels, jpeg_quality) == 0;
    if (channels == 1)
        return bitgrain_write_pgm(cur_out, pixels, width, height) == 0;
    return 0;
}

int decode_cli_run(const cli_ctx_t *ctx)
{
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
            size_t stem_len = (base_len >= 3 && platform_strcasecmp(base + base_len - 3, ".bg") == 0) ? base_len - 3 : base_len;
            cur_out_owned = (char *)malloc(dlen + stem_len + 6);
            if (!cur_out_owned) { fprintf(stderr, "Error: out of memory.\n"); dec_failed = 1; break; }
            snprintf(cur_out_owned, dlen + stem_len + 6, "%s/%.*s.jpg", ctx->output_dir, (int)stem_len, base);
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

        FILE *f = fopen(cur_in, "rb");
        if (!f) {
            fprintf(stderr, "Error: could not open '%s'.\n", cur_in);
            free(cur_out_owned);
            dec_failed = 1;
            if (!ctx->multi) break;
            continue;
        }
        if (fseek(f, 0, SEEK_END) != 0) {
            fclose(f);
            free(cur_out_owned);
            dec_failed = 1;
            if (!ctx->multi) break;
            continue;
        }
        long fsize = ftell(f);
        if (fsize <= 0 || fsize > (long)BITGRAIN_MAX_BG_FILE) {
            fclose(f);
            free(cur_out_owned);
            fprintf(stderr, "Error: .bg file invalid or too large '%s'.\n", cur_in);
            dec_failed = 1;
            if (!ctx->multi) break;
            continue;
        }
        rewind(f);
        uint8_t *bg_buf = (uint8_t *)malloc((size_t)fsize);
        if (!bg_buf) {
            fclose(f);
            free(cur_out_owned);
            dec_failed = 1;
            if (!ctx->multi) break;
            continue;
        }
        if (fread(bg_buf, 1, (size_t)fsize, f) != (size_t)fsize) {
            fclose(f);
            free(bg_buf);
            free(cur_out_owned);
            dec_failed = 1;
            if (!ctx->multi) break;
            continue;
        }
        fclose(f);

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
        int ret = bitgrain_decode(bg_buf, (int32_t)fsize, pixels, (int32_t)pixel_bytes, &width, &height, &channels);
        free(bg_buf);
        if (ret != 0) {
            fprintf(stderr, "Error: '%s' is not a valid .bg or is corrupt.\n", cur_in);
            free(pixels);
            free(cur_out_owned);
            dec_failed = 1;
            if (!ctx->multi) break;
            continue;
        }
        if (!write_output(cur_out, pixels, width, height, channels, ctx->jpeg_out_quality)) {
            fprintf(stderr, "Error: could not write '%s'.\n", cur_out);
            free(pixels);
            free(cur_out_owned);
            dec_failed = 1;
            if (!ctx->multi) break;
            continue;
        }
        printf("%s -> %s  (%u×%u, %u ch)\n", cur_in, cur_out, width, height, channels);
        free(pixels);
        free(cur_out_owned);
    }
    return dec_failed ? 1 : 0;
}
