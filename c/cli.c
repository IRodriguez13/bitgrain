/* SPDX-License-Identifier: GPL-3.0-or-later */
#define _POSIX_C_SOURCE 200809L
#include "cli.h"
#include "platform.h"
#include "config.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static char output_dir_buf[1024];

static void set_output_dir(const char *p)
{
    size_t len = strlen(p);
    while (len > 0 && (p[len - 1] == '/' || p[len - 1] == '\\')) len--;
    if (len >= sizeof(output_dir_buf)) len = sizeof(output_dir_buf) - 1;
    memcpy(output_dir_buf, p, len);
    output_dir_buf[len] = '\0';
}

void cli_usage(const char *prog)
{
    fprintf(stderr,
            "bitgrain " BITGRAIN_VERSION " – image compressor (JPEG-like .bg format)\n"
            "  encode: image → .bg   decode: .bg → image   round-trip: image → image (no .bg file)\n\n"
            "Usage:\n"
            "  %s -i <in> -o <out>       encode image to .bg\n"
            "  %s <image>               → <image>.bg\n"
            "  %s -d -i <file.bg> -o <out>   decode .bg to image (.jpg/.png/.pgm by -o)\n"
            "  %s -cd -i <image> -o <out>    round-trip: compress+decompress in memory\n"
            "  %s -cd -o <outdir> <img1> <img2> ...   multiple images (or a directory)\n\n"
            "Options:\n"
            "  -i <path>   input file or directory (with -cd/encode: all images inside)\n"
            "  -o <path>   output file or directory (directory required for multiple inputs)\n"
            "  -d          decode (.bg → image)\n"
            "  -cd         round-trip (no .bg file written)\n"
            "  -q <1-100>  .bg encode quality (default 85)\n"
            "  -Q <1-100>  output JPG quality when writing .jpg (default 85; smaller file)\n"
            "  -m          round-trip: print PSNR/SSIM (quality vs original)\n"
            "  -y          overwrite\n"
            "  -v          version\n"
            "  -h          help\n",
            prog, prog, prog, prog, prog);
}

int cli_parse(int argc, char **argv, cli_ctx_t *ctx)
{
    const char *input_path = NULL;
    const char *output_path = NULL;
    int opt;

    memset(ctx, 0, sizeof(*ctx));
    ctx->quality = 85;
    ctx->jpeg_out_quality = 85;

    while ((opt = getopt(argc, argv, "i:o:cdq:Q:myvh")) != -1) {
        switch (opt) {
        case 'i': input_path = optarg; break;
        case 'o': output_path = optarg; break;
        case 'c': ctx->round_trip = 1; break;
        case 'd': if (!ctx->round_trip) ctx->decode_mode = 1; break;
        case 'q':
            ctx->quality = atoi(optarg);
            if (ctx->quality < 1) ctx->quality = 1;
            if (ctx->quality > 100) ctx->quality = 100;
            break;
        case 'Q':
            ctx->jpeg_out_quality = atoi(optarg);
            if (ctx->jpeg_out_quality < 1) ctx->jpeg_out_quality = 1;
            if (ctx->jpeg_out_quality > 100) ctx->jpeg_out_quality = 100;
            break;
        case 'm': ctx->show_metrics = 1; break;
        case 'y': ctx->overwrite = 1; break;
        case 'v':
            printf("bitgrain %s\n", BITGRAIN_VERSION);
            printf("Author: Iván E. Rodriguez\n");
            printf("License: GPLv3\n");
            printf("Upstream: https://github.com/IRodriguez13/bitgrain\n");
            return -2; /* special: exit 0 */
        case 'h':
            cli_usage(argv[0]);
            return -2;
        default:
            cli_usage(argv[0]);
            return -1;
        }
    }

    path_list_t input_specs = { NULL, 0, 0 };
    if (input_path) path_list_push(&input_specs, input_path);
    for (int k = optind; k < argc; k++)
        if (argv[k][0] != '-')
            path_list_push(&input_specs, argv[k]);

    if (input_specs.n == 0) {
        fprintf(stderr, "Error: missing input (use -i <file|dir> and/or list files/dirs).\n");
        cli_usage(argv[0]);
        return -1;
    }

    /* Expand specs */
    int expand_bg_only = ctx->decode_mode;
    for (size_t k = 0; k < input_specs.n; k++) {
        if (path_list_append_from_spec(&ctx->expanded, input_specs.paths[k], expand_bg_only) != 0)
            fprintf(stderr, "Warning: skipping invalid or unreadable path '%s'.\n", input_specs.paths[k]);
    }
    path_list_free(&input_specs);

    if (ctx->expanded.n == 0) {
        fprintf(stderr, "Error: no %s found in the given path(s).\n",
                ctx->decode_mode ? ".bg files" : "image files");
        cli_usage(argv[0]);
        return -1;
    }

    ctx->multi = (ctx->expanded.n > 1);

    if (ctx->multi) {
        if (output_path) {
            int out_is_dir = 0, out_is_reg = 0;
            if (platform_stat(output_path, &out_is_dir, &out_is_reg) == 0) {
                if (out_is_dir)
                    set_output_dir(output_path);
                else {
                    fprintf(stderr, "Error: with multiple inputs -o must be a directory (e.g. -o out).\n");
                    path_list_free(&ctx->expanded);
                    return -1;
                }
            } else {
                if (platform_mkdir(output_path) == 0)
                    set_output_dir(output_path);
                else {
                    fprintf(stderr, "Error: could not create output directory '%s'.\n", output_path);
                    path_list_free(&ctx->expanded);
                    return -1;
                }
            }
            ctx->output_dir = output_dir_buf;
        } else {
            ctx->output_dir_owned = strdup("out");
            ctx->output_dir = ctx->output_dir_owned;
            (void)platform_mkdir(ctx->output_dir);
        }
    } else {
        const char *cur_in = ctx->expanded.paths[0];
        if (!output_path) {
            char def[1024];
            if (default_output_path(cur_in, def, sizeof(def), ctx->decode_mode, ctx->round_trip) != 0) {
                fprintf(stderr, "Error: input path too long.\n");
                path_list_free(&ctx->expanded);
                return -1;
            }
            ctx->output_path_owned = strdup(def);
            ctx->output_path = ctx->output_path_owned;
        } else {
            ctx->output_path = output_path;
        }
        if (!ctx->overwrite) {
            FILE *exists = fopen(ctx->output_path, "rb");
            if (exists) {
                fclose(exists);
                char *alt = avoid_overwrite_path(ctx->output_path);
                if (alt) {
                    free(ctx->output_path_owned);
                    ctx->output_path_owned = alt;
                    ctx->output_path = ctx->output_path_owned;
                } else {
                    fprintf(stderr, "Error: '%s' already exists. Use -y to overwrite.\n", ctx->output_path);
                    path_list_free(&ctx->expanded);
                    return -1;
                }
            }
        }
    }

    return 0;
}

void cli_ctx_free(cli_ctx_t *ctx)
{
    path_list_free(&ctx->expanded);
    free(ctx->output_path_owned);
    free(ctx->output_dir_owned);
}
