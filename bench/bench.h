/* SPDX-License-Identifier: GPL-3.0-or-later */
/* bench.h — Bitgrain profiling & benchmarking framework.
 * Standalone: no dependency on bitgrain CLI internals.
 * Links only against libbitgrain (encoder/decoder FFI) and libc. */
#ifndef BITGRAIN_BENCH_H
#define BITGRAIN_BENCH_H

#include <stdint.h>
#include <stddef.h>
#include <stdio.h>

#ifdef __cplusplus
extern "C" {
#endif

/* ------------------------------------------------------------------ */
/* Timer                                                                */
/* ------------------------------------------------------------------ */

typedef struct {
    uint64_t start_ns;
} bg_timer_t;

void     bg_timer_start(bg_timer_t *t);
uint64_t bg_timer_elapsed_ns(const bg_timer_t *t);
double   bg_timer_elapsed_ms(const bg_timer_t *t);

/* ------------------------------------------------------------------ */
/* Result for a single benchmark run                                    */
/* ------------------------------------------------------------------ */

typedef struct {
    const char *label;
    double      encode_ms;
    double      decode_ms;
    double      total_ms;
    size_t      input_bytes;
    size_t      output_bytes;
    double      ratio;          /* output / input */
    double      encode_mpps;    /* megapixels/sec encode */
    double      decode_mpps;    /* megapixels/sec decode */
    double      psnr;           /* -1 if not computed */
    double      ssim;           /* -1 if not computed */
    int         ok;
} bg_bench_result_t;

/* ------------------------------------------------------------------ */
/* Benchmark configuration                                              */
/* ------------------------------------------------------------------ */

typedef struct {
    const char *image_path;     /* input image file */
    int         quality;        /* 1–100 */
    int         warmup_runs;    /* runs before timing (default 1) */
    int         timed_runs;     /* runs to average (default 5) */
    int         compute_metrics;/* PSNR/SSIM (default 1) */
    int         verbose;        /* print per-run timing */
} bg_bench_config_t;

void bg_bench_config_defaults(bg_bench_config_t *cfg);

/* ------------------------------------------------------------------ */
/* Run a full encode+decode benchmark for one image/quality pair.      */
/* ------------------------------------------------------------------ */

bg_bench_result_t bg_bench_run(const bg_bench_config_t *cfg);

/* ------------------------------------------------------------------ */
/* Report printing                                                      */
/* ------------------------------------------------------------------ */

/* Print a single result row (for table output). */
void bg_bench_print_row(FILE *f, const bg_bench_result_t *r);

/* Print a full report header. */
void bg_bench_print_header(FILE *f);

/* Print a summary separator. */
void bg_bench_print_separator(FILE *f);

/* Print a full JSON report for all results. */
void bg_bench_print_json(FILE *f, const bg_bench_result_t *results, size_t n);

#ifdef __cplusplus
}
#endif

#endif /* BITGRAIN_BENCH_H */
