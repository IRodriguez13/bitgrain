/* SPDX-License-Identifier: GPL-3.0-or-later */
#include "roundtrip_cli.h"
#include "encoder.h"
#include "image_loader.h"
#include "config.h"
#include "image_writer.h"
#include "icc_io.h"
#include "webp_io.h"
#include "metrics.h"
#include "bg_utils.h"
#include "path_utils.h"
#include "platform.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <strings.h>

static int write_output(const char *cur_out, const uint8_t *decoded,
                        uint32_t width, uint32_t height, uint32_t channels,
                        int jpeg_quality, const uint8_t *icc, uint32_t icc_len)
{
    size_t len = strlen(cur_out);
    if (channels >= 1 && channels <= 4 && len >= 4 && (strcmp(cur_out + len - 4, ".jpg") == 0 ||
            (len >= 5 && strcmp(cur_out + len - 5, ".jpeg") == 0)))
        return bitgrain_write_jpg(cur_out, decoded, width, height, (int)channels, jpeg_quality) == 0;
    if (len >= 4 && strcmp(cur_out + len - 4, ".png") == 0) {
        if (icc && icc_len > 0) {
            return bitgrain_write_png_with_icc(cur_out, decoded, width, height, (int)channels, icc, icc_len) == 0;
        }
        return bitgrain_write_png(cur_out, decoded, width, height, (int)channels) == 0;
    }
    if (len >= 5 && strcmp(cur_out + len - 5, ".webp") == 0)
        return bitgrain_write_webp(cur_out, decoded, width, height, (int)channels, jpeg_quality) == 0;
    if (channels == 1)
        return bitgrain_write_pgm(cur_out, decoded, width, height) == 0;
    return 0;
}

int roundtrip_cli_run(const cli_ctx_t *ctx)
{
    int rt_failed = 0;
    for (size_t idx = 0; idx < ctx->expanded.n; idx++) {
        const char *cur_in = ctx->expanded.paths[idx];
        char *cur_out_owned = NULL;
        const char *cur_out;

        if (ctx->multi) {
            const char *base = strrchr(cur_in, '/');
            base = base ? base + 1 : cur_in;
            size_t dlen = strlen(ctx->output_dir);
            size_t blen = strlen(base);
            const char *dot = strrchr(base, '.');
            size_t stem_len = dot ? (size_t)(dot - base) : blen;
            size_t ext_len = dot ? strlen(dot) : 4u;  /* .png */
            size_t out_cap = dlen + 1 + stem_len + ext_len + 1;
            cur_out_owned = (char *)malloc(out_cap);
            if (!cur_out_owned) { fprintf(stderr, "Error: out of memory.\n"); rt_failed = 1; break; }
            snprintf(cur_out_owned, out_cap, "%s/%.*s%s", ctx->output_dir, (int)stem_len, base, dot ? dot : ".png");
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

        uint32_t width, height, channels;
        uint8_t *icc_in = NULL;
        uint32_t icc_in_len = 0;
        const char *dot_in = strrchr(cur_in, '.');
        if (dot_in && (strcasecmp(dot_in, ".png") == 0)) {
            icc_in = bitgrain_load_icc_from_png(cur_in, &icc_in_len);
        }
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
            rt_failed = 1;
            if (!ctx->multi) break;
            continue;
        }
        if (check_image_size(width, height, channels) != 0) {
            fprintf(stderr, "Error: image too large '%s' (max %u×%u).\n", cur_in, BITGRAIN_MAX_DIM, BITGRAIN_MAX_DIM);
            bitgrain_image_free(pixels);
            free(cur_out_owned);
            rt_failed = 1;
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
            rt_failed = 1;
            if (!ctx->multi) break;
            continue;
        }

        int32_t out_len = 0;
        int ret;
        if (channels == 4)
            ret = icc_in ? bitgrain_encode_rgba_icc(pixels, width, height, out_buf, (int32_t)out_buf_size, &out_len, (uint8_t)ctx->quality, icc_in, icc_in_len)
                         : bitgrain_encode_rgba(pixels, width, height, out_buf, (int32_t)out_buf_size, &out_len, (uint8_t)ctx->quality);
        else if (channels == 3)
            ret = icc_in ? bitgrain_encode_rgb_icc(pixels, width, height, out_buf, (int32_t)out_buf_size, &out_len, (uint8_t)ctx->quality, icc_in, icc_in_len)
                         : bitgrain_encode_rgb(pixels, width, height, out_buf, (int32_t)out_buf_size, &out_len, (uint8_t)ctx->quality);
        else
            ret = bitgrain_encode_grayscale(pixels, width, height, out_buf, (int32_t)out_buf_size, &out_len, (uint8_t)ctx->quality);
        if (ret != 0) {
            fprintf(stderr, "Error: encode failed '%s'.\n", cur_in);
            bitgrain_image_free(pixels);
            free(out_buf);
            free(cur_out_owned);
            rt_failed = 1;
            if (!ctx->multi) break;
            continue;
        }

        size_t decoded_size = (size_t)raw_bytes;
        uint8_t *decoded = (uint8_t *)malloc(decoded_size);
        if (!decoded) {
            bitgrain_image_free(pixels);
            free(out_buf);
            free(cur_out_owned);
            rt_failed = 1;
            if (!ctx->multi) break;
            continue;
        }
        uint8_t *icc_out = NULL;
        uint32_t icc_out_len = 0;
        ret = bitgrain_decode_icc(out_buf, out_len, decoded, (int32_t)decoded_size, &width, &height, &channels, &icc_out, &icc_out_len);
        free(out_buf);
        if (icc_in) free(icc_in);
        if (ret != 0) {
            fprintf(stderr, "Error: decode failed '%s'.\n", cur_in);
            bitgrain_image_free(pixels);
            free(decoded);
            free(cur_out_owned);
            rt_failed = 1;
            if (!ctx->multi) break;
            continue;
        }

        if (ctx->show_metrics) {
            double psnr = bitgrain_psnr(pixels, decoded, width, height, channels);
            double ssim = bitgrain_ssim(pixels, decoded, width, height, channels);
            printf("  PSNR %.2f dB  SSIM %.4f\n", psnr, ssim);
        }

        if (!write_output(cur_out, decoded, width, height, channels, ctx->jpeg_out_quality, icc_out, icc_out_len)) {
            fprintf(stderr, "Error: could not write '%s' (use .jpg, .png, .pgm, .webp).\n", cur_out);
            free(decoded);
            bitgrain_image_free(pixels);
            free(cur_out_owned);
            rt_failed = 1;
            if (!ctx->multi) break;
            continue;
        }
        printf("%s -> %s  (%u×%u, round-trip)\n", cur_in, cur_out, width, height);
        if (icc_out) bitgrain_free_icc(icc_out, icc_out_len);
        free(decoded);
        bitgrain_image_free(pixels);
        free(cur_out_owned);
    }
    return rt_failed ? 1 : 0;
}
