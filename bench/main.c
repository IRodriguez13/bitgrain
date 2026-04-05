/* SPDX-License-Identifier: GPL-3.0-or-later */
/* bitgrain-bench — standalone profiling tool. */

#define _POSIX_C_SOURCE 200809L
#include "bench.h"
#include "../includes/encoder.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <dirent.h>
#include <sys/stat.h>
#include <strings.h>

#define MAX_IMAGES   256
#define MAX_QUALITIES 16

static void print_help(const char *prog)
{
    fprintf(stderr,
        "bitgrain-bench %s — image codec profiler\n\n"
        "Usage:\n"
        "  %s [options] <image> [image2 ...]\n\n"
        "Options:\n"
        "  -q <1-100>       Encode quality (default 85; can repeat for sweep)\n"
        "  -r <n>           Timed runs per pair (default 5)\n"
        "  -w <n>           Warmup runs (default 1)\n"
        "  -t <n>           Worker threads (default runtime)\n"
        "  --no-metrics     Skip PSNR/SSIM computation\n"
        "  --verbose        Print per-run timings to stderr\n"
        "  --json           Output JSON to stdout\n"
        "  --json-file <f>  Write JSON report to file\n"
        "  -h / --help      This help\n\n"
        "Examples:\n"
        "  %s img/photo.jpg\n"
        "  %s -q 50 -q 75 -q 90 img/photo.jpg img/other.png\n"
        "  %s -r 10 --json img/photo.jpg > report.json\n"
        "  %s img/ -q 85 --no-metrics\n",
        "1.0.0", prog, prog, prog, prog, prog);
}

/* Collect image paths from a directory (non-recursive, image extensions only). */
static int collect_dir(const char *dir, const char **out, int max)
{
    static const char *EXTS[] = {
        ".jpg", ".jpeg", ".png", ".bmp", ".tga", ".pgm", ".ppm", ".pnm", ".pbm",
        ".tif", ".tiff", ".webp", NULL
    };
    int count = 0;
    DIR *d = opendir(dir);
    if (!d) return 0;
    struct dirent *ent;
    while ((ent = readdir(d)) != NULL && count < max) {
        if (ent->d_name[0] == '.') continue;
        const char *dot = strrchr(ent->d_name, '.');
        if (!dot) continue;
        int match = 0;
        for (int i = 0; EXTS[i]; i++) {
            if (strcasecmp(dot, EXTS[i]) == 0) { match = 1; break; }
        }
        if (!match) continue;
        size_t dlen = strlen(dir);
        size_t nlen = strlen(ent->d_name);
        char *full = (char *)malloc(dlen + nlen + 2);
        if (!full) continue;
        snprintf(full, dlen + nlen + 2, "%s/%s", dir, ent->d_name);
        out[count++] = full;
    }
    closedir(d);
    return count;
}

static int is_dir(const char *path)
{
    struct stat st;
    return (stat(path, &st) == 0 && S_ISDIR(st.st_mode));
}

int main(int argc, char **argv)
{
    if (argc < 2) {
        print_help(argv[0]);
        return 1;
    }

    const char *images[MAX_IMAGES];
    int         n_images = 0;
    int         qualities[MAX_QUALITIES];
    int         n_qualities = 0;
    int         timed_runs  = 5;
    int         warmup_runs = 1;
    int         metrics     = 1;
    int         verbose     = 0;
    int         threads     = 0;
    int         json_stdout = 0;
    const char *json_file   = NULL;

    /* Parse args */
    for (int i = 1; i < argc; i++) {
        const char *a = argv[i];

        if (strcmp(a, "-h") == 0 || strcmp(a, "--help") == 0) {
            print_help(argv[0]);
            return 0;
        }
        if (strcmp(a, "--no-metrics") == 0) { metrics = 0; continue; }
        if (strcmp(a, "--verbose")    == 0) { verbose = 1; continue; }
        if (strcmp(a, "--json")       == 0) { json_stdout = 1; continue; }

        if (strcmp(a, "--json-file") == 0 && i + 1 < argc) {
            json_file = argv[++i]; continue;
        }
        if (strcmp(a, "-q") == 0 && i + 1 < argc) {
            int q = atoi(argv[++i]);
            if (q < 1) q = 1;
            if (q > 100) q = 100;
            if (n_qualities < MAX_QUALITIES) qualities[n_qualities++] = q;
            continue;
        }
        if (strcmp(a, "-r") == 0 && i + 1 < argc) {
            timed_runs = atoi(argv[++i]);
            if (timed_runs < 1) timed_runs = 1;
            continue;
        }
        if (strcmp(a, "-t") == 0 && i + 1 < argc) {
            threads = atoi(argv[++i]);
            if (threads < 1) threads = 1;
            continue;
        }
        if (strcmp(a, "-w") == 0 && i + 1 < argc) {
            warmup_runs = atoi(argv[++i]);
            if (warmup_runs < 0) warmup_runs = 0;
            continue;
        }
        if (a[0] == '-') {
            fprintf(stderr, "Unknown option: %s\n", a);
            return 1;
        }

        /* Positional: image file or directory */
        if (is_dir(a)) {
            n_images += collect_dir(a, images + n_images, MAX_IMAGES - n_images);
        } else {
            if (n_images < MAX_IMAGES) images[n_images++] = a;
        }
    }

    if (n_images == 0) {
        fprintf(stderr, "Error: no images specified.\n");
        print_help(argv[0]);
        return 1;
    }
    if (n_qualities == 0) { qualities[0] = 85; n_qualities = 1; }

    if (threads > 0) {
        if (bitgrain_set_threads(threads) != 0) {
            fprintf(stderr, "Error: could not configure thread pool to %d\n", threads);
            return 1;
        }
    }

    int n_results = n_images * n_qualities;
    bg_bench_result_t *results = (bg_bench_result_t *)calloc(n_results, sizeof(*results));
    if (!results) { fprintf(stderr, "out of memory\n"); return 1; }

    /* ---- Run benchmarks ---- */
    if (!json_stdout) {
        fprintf(stdout, "\nbitgrain-bench  (runs=%d, warmup=%d, metrics=%s)\n\n",
                timed_runs, warmup_runs, metrics ? "yes" : "no");
        bg_bench_print_header(stdout);
        bg_bench_print_separator(stdout);
    }

    int ri = 0;
    for (int qi = 0; qi < n_qualities; qi++) {
        for (int ii = 0; ii < n_images; ii++, ri++) {
            bg_bench_config_t cfg;
            bg_bench_config_defaults(&cfg);
            cfg.image_path      = images[ii];
            cfg.quality         = qualities[qi];
            cfg.timed_runs      = timed_runs;
            cfg.warmup_runs     = warmup_runs;
            cfg.compute_metrics = metrics;
            cfg.verbose         = verbose;

            results[ri] = bg_bench_run(&cfg);

            if (!json_stdout) {
                /* Print row with quality injected */
                const bg_bench_result_t *r = &results[ri];
                if (!r->ok) {
                    fprintf(stdout, "  FAILED: %s\n", images[ii]);
                    continue;
                }
                const char *label = r->label ? r->label : "?";
                const char *slash = strrchr(label, '/');
                if (slash) label = slash + 1;

                char psnr_buf[16] = "   n/a", ssim_buf[16] = "   n/a";
                if (r->psnr >= 0) snprintf(psnr_buf, sizeof(psnr_buf), "%6.2f", r->psnr);
                if (r->ssim >= 0) snprintf(ssim_buf, sizeof(ssim_buf), "%6.4f", r->ssim);

                fprintf(stdout,
                    "%-22s  %5d  %8.2f  %8.2f  %8.2f  %8.1f  %8.1f  %7.3f  %s  %s\n",
                    label, qualities[qi],
                    r->encode_ms, r->decode_ms, r->total_ms,
                    r->input_bytes  / 1024.0,
                    r->output_bytes / 1024.0,
                    r->ratio,
                    psnr_buf, ssim_buf);
                fflush(stdout);
            }
        }
    }

    if (!json_stdout) {
        bg_bench_print_separator(stdout);
        /* Summary: averages */
        double avg_enc = 0, avg_dec = 0, avg_psnr = 0, avg_ssim = 0;
        int    ok_count = 0, metric_count = 0;
        for (int i = 0; i < n_results; i++) {
            if (!results[i].ok) continue;
            avg_enc  += results[i].encode_ms;
            avg_dec  += results[i].decode_ms;
            ok_count++;
            if (results[i].psnr >= 0) {
                avg_psnr += results[i].psnr;
                avg_ssim += results[i].ssim;
                metric_count++;
            }
        }
        if (ok_count > 0) {
            fprintf(stdout, "\nAverage (n=%d):  enc %.2f ms  dec %.2f ms",
                    ok_count, avg_enc / ok_count, avg_dec / ok_count);
            if (metric_count > 0)
                fprintf(stdout, "  PSNR %.2f dB  SSIM %.4f",
                        avg_psnr / metric_count, avg_ssim / metric_count);
            fprintf(stdout, "\n");
        }
    }

    /* JSON output */
    if (json_stdout) {
        bg_bench_print_json(stdout, results, (size_t)n_results);
    }
    if (json_file) {
        FILE *jf = fopen(json_file, "w");
        if (jf) {
            bg_bench_print_json(jf, results, (size_t)n_results);
            fclose(jf);
            if (!json_stdout)
                fprintf(stdout, "\nJSON report written to: %s\n", json_file);
        } else {
            fprintf(stderr, "Warning: could not write JSON to '%s'\n", json_file);
        }
    }

    free(results);
    return 0;
}
