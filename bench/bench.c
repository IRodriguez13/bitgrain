/* SPDX-License-Identifier: GPL-3.0-or-later */
/* bench.c — Bitgrain benchmark core implementation. */

#define _POSIX_C_SOURCE 200809L
#include "bench.h"
#include "../includes/encoder.h"
#include "../c/metrics.h"

#include <stdlib.h>
#include <string.h>
#include <math.h>
#include <time.h>

/* ------------------------------------------------------------------ */
/* stb_image for loading (header-only, standalone copy)                */
/* ------------------------------------------------------------------ */
#define STB_IMAGE_IMPLEMENTATION
#define STB_IMAGE_STATIC
#define STBI_NO_HDR
#define STBI_NO_PIC
#if defined(__GNUC__)
#pragma GCC diagnostic push
#pragma GCC diagnostic ignored "-Wunused-function"
#pragma GCC diagnostic ignored "-Wstringop-overflow"
#endif
#include "../c/stb_image.h"
#if defined(__GNUC__)
#pragma GCC diagnostic pop
#endif

/* ------------------------------------------------------------------ */
/* Timer                                                                */
/* ------------------------------------------------------------------ */

void bg_timer_start(bg_timer_t *t)
{
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    t->start_ns = (uint64_t)ts.tv_sec * 1000000000ULL + (uint64_t)ts.tv_nsec;
}

uint64_t bg_timer_elapsed_ns(const bg_timer_t *t)
{
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    uint64_t now = (uint64_t)ts.tv_sec * 1000000000ULL + (uint64_t)ts.tv_nsec;
    return now - t->start_ns;
}

double bg_timer_elapsed_ms(const bg_timer_t *t)
{
    return (double)bg_timer_elapsed_ns(t) / 1e6;
}

/* ------------------------------------------------------------------ */
/* Config defaults                                                      */
/* ------------------------------------------------------------------ */

void bg_bench_config_defaults(bg_bench_config_t *cfg)
{
    cfg->image_path      = NULL;
    cfg->quality         = 85;
    cfg->warmup_runs     = 1;
    cfg->timed_runs      = 5;
    cfg->compute_metrics = 1;
    cfg->verbose         = 0;
}

/* ------------------------------------------------------------------ */
/* Core benchmark                                                       */
/* ------------------------------------------------------------------ */

bg_bench_result_t bg_bench_run(const bg_bench_config_t *cfg)
{
    bg_bench_result_t res;
    memset(&res, 0, sizeof(res));
    res.label = cfg->image_path;
    res.psnr  = -1.0;
    res.ssim  = -1.0;
    res.ok    = 0;

    /* Load image */
    int w, h, n;
    unsigned char *pixels_orig = stbi_load(cfg->image_path, &w, &h, &n, 0);
    if (!pixels_orig) {
        fprintf(stderr, "[bench] could not load '%s'\n", cfg->image_path);
        return res;
    }

    uint32_t width    = (uint32_t)w;
    uint32_t height   = (uint32_t)h;
    uint32_t channels = (uint32_t)n;
    size_t   raw_size = (size_t)w * h * n;

    /* Allocate encode buffer (2x raw + margin) */
    size_t enc_cap = raw_size * 2 + 1024 * 1024;
    uint8_t *enc_buf = (uint8_t *)malloc(enc_cap);
    uint8_t *dec_buf = (uint8_t *)malloc(raw_size);
    if (!enc_buf || !dec_buf) {
        fprintf(stderr, "[bench] out of memory\n");
        stbi_image_free(pixels_orig);
        free(enc_buf); free(dec_buf);
        return res;
    }

    /* ---- Warmup ---- */
    for (int i = 0; i < cfg->warmup_runs; i++) {
        int32_t out_len = 0;
        if (channels == 4)
            bitgrain_encode_rgba(pixels_orig, width, height, enc_buf, (uint32_t)enc_cap, &out_len, (uint8_t)cfg->quality);
        else if (channels == 3)
            bitgrain_encode_rgb(pixels_orig, width, height, enc_buf, (uint32_t)enc_cap, &out_len, (uint8_t)cfg->quality);
        else
            bitgrain_encode_grayscale(pixels_orig, width, height, enc_buf, (uint32_t)enc_cap, &out_len, (uint8_t)cfg->quality);
        uint32_t dw, dh, dc;
        bitgrain_decode(enc_buf, out_len, dec_buf, (uint32_t)raw_size, &dw, &dh, &dc);
    }

    /* ---- Timed encode runs ---- */
    double total_enc_ms = 0.0;
    int32_t final_enc_len = 0;
    for (int i = 0; i < cfg->timed_runs; i++) {
        int32_t out_len = 0;
        bg_timer_t t;
        bg_timer_start(&t);
        int ret;
        if (channels == 4)
            ret = bitgrain_encode_rgba(pixels_orig, width, height, enc_buf, (uint32_t)enc_cap, &out_len, (uint8_t)cfg->quality);
        else if (channels == 3)
            ret = bitgrain_encode_rgb(pixels_orig, width, height, enc_buf, (uint32_t)enc_cap, &out_len, (uint8_t)cfg->quality);
        else
            ret = bitgrain_encode_grayscale(pixels_orig, width, height, enc_buf, (uint32_t)enc_cap, &out_len, (uint8_t)cfg->quality);
        double ms = bg_timer_elapsed_ms(&t);
        if (ret != 0) {
            fprintf(stderr, "[bench] encode failed run %d\n", i);
            goto cleanup;
        }
        total_enc_ms += ms;
        final_enc_len = out_len;
        if (cfg->verbose)
            fprintf(stderr, "  encode run %d: %.3f ms\n", i + 1, ms);
    }

    /* ---- Timed decode runs ---- */
    double total_dec_ms = 0.0;
    for (int i = 0; i < cfg->timed_runs; i++) {
        uint32_t dw = 0, dh = 0, dc = 0;
        bg_timer_t t;
        bg_timer_start(&t);
        int ret = bitgrain_decode(enc_buf, final_enc_len, dec_buf, (uint32_t)raw_size, &dw, &dh, &dc);
        double ms = bg_timer_elapsed_ms(&t);
        if (ret != 0) {
            fprintf(stderr, "[bench] decode failed run %d\n", i);
            goto cleanup;
        }
        total_dec_ms += ms;
        if (cfg->verbose)
            fprintf(stderr, "  decode run %d: %.3f ms\n", i + 1, ms);
    }

    /* ---- Metrics ---- */
    if (cfg->compute_metrics) {
        /* Decode once more to get final pixels */
        uint32_t dw = 0, dh = 0, dc = 0;
        bitgrain_decode(enc_buf, final_enc_len, dec_buf, (uint32_t)raw_size, &dw, &dh, &dc);
        res.psnr = bitgrain_psnr(pixels_orig, dec_buf, width, height, channels);
        res.ssim = bitgrain_ssim(pixels_orig, dec_buf, width, height, channels);
    }

    /* ---- Fill result ---- */
    {
        double runs = (double)cfg->timed_runs;
        double mpix = (double)width * height / 1e6;
        res.encode_ms   = total_enc_ms / runs;
        res.decode_ms   = total_dec_ms / runs;
        res.total_ms    = res.encode_ms + res.decode_ms;
        res.input_bytes = raw_size;
        res.output_bytes = (size_t)final_enc_len;
        res.ratio        = (raw_size > 0) ? (double)final_enc_len / (double)raw_size : 0.0;
        res.encode_mpps  = (res.encode_ms > 0) ? mpix / (res.encode_ms / 1000.0) : 0.0;
        res.decode_mpps  = (res.decode_ms > 0) ? mpix / (res.decode_ms / 1000.0) : 0.0;
        res.ok = 1;
    }

cleanup:
    stbi_image_free(pixels_orig);
    free(enc_buf);
    free(dec_buf);
    return res;
}

/* ------------------------------------------------------------------ */
/* Report printing                                                      */
/* ------------------------------------------------------------------ */

#define COL_W 22

void bg_bench_print_header(FILE *f)
{
    fprintf(f,
        "%-*s  %5s  %8s  %8s  %8s  %8s  %8s  %8s  %7s  %7s\n",
        COL_W, "Image",
        "Q",
        "Enc(ms)", "Dec(ms)", "Tot(ms)",
        "InKB", "OutKB", "Ratio",
        "PSNR", "SSIM");
}

void bg_bench_print_separator(FILE *f)
{
    for (int i = 0; i < COL_W + 2 + 5 + 2 + 8*6 + 7*2 + 20; i++) fputc('-', f);
    fputc('\n', f);
}

void bg_bench_print_row(FILE *f, const bg_bench_result_t *r)
{
    if (!r->ok) {
        fprintf(f, "%-*s  FAILED\n", COL_W, r->label ? r->label : "?");
        return;
    }

    /* Shorten label to last path component */
    const char *label = r->label ? r->label : "?";
    const char *slash = strrchr(label, '/');
    if (slash) label = slash + 1;

    char psnr_buf[16], ssim_buf[16];
    if (r->psnr >= 0) snprintf(psnr_buf, sizeof(psnr_buf), "%6.2f", r->psnr);
    else              snprintf(psnr_buf, sizeof(psnr_buf), "   n/a");
    if (r->ssim >= 0) snprintf(ssim_buf, sizeof(ssim_buf), "%6.4f", r->ssim);
    else              snprintf(ssim_buf, sizeof(ssim_buf), "   n/a");

    fprintf(f,
        "%-*s  %5d  %8.2f  %8.2f  %8.2f  %8.1f  %8.1f  %7.3f  %s  %s\n",
        COL_W, label,
        (int)((r->psnr >= 0) ? 0 : 0),   /* placeholder for quality — filled by caller */
        r->encode_ms, r->decode_ms, r->total_ms,
        r->input_bytes  / 1024.0,
        r->output_bytes / 1024.0,
        r->ratio,
        psnr_buf, ssim_buf);
}

void bg_bench_print_json(FILE *f, const bg_bench_result_t *results, size_t n)
{
    fprintf(f, "[\n");
    for (size_t i = 0; i < n; i++) {
        const bg_bench_result_t *r = &results[i];
        const char *label = r->label ? r->label : "";
        fprintf(f,
            "  {\n"
            "    \"image\": \"%s\",\n"
            "    \"ok\": %s,\n"
            "    \"encode_ms\": %.4f,\n"
            "    \"decode_ms\": %.4f,\n"
            "    \"total_ms\": %.4f,\n"
            "    \"input_bytes\": %zu,\n"
            "    \"output_bytes\": %zu,\n"
            "    \"ratio\": %.6f,\n"
            "    \"encode_mpps\": %.4f,\n"
            "    \"decode_mpps\": %.4f,\n"
            "    \"psnr\": %.4f,\n"
            "    \"ssim\": %.6f\n"
            "  }%s\n",
            label,
            r->ok ? "true" : "false",
            r->encode_ms, r->decode_ms, r->total_ms,
            r->input_bytes, r->output_bytes,
            r->ratio,
            r->encode_mpps, r->decode_mpps,
            r->psnr, r->ssim,
            (i + 1 < n) ? "," : "");
    }
    fprintf(f, "]\n");
}
