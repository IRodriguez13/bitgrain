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

/* ------------------------------------------------------------------ */
/* Help per subcommand                                                  */
/* ------------------------------------------------------------------ */

static void usage_encode(const char *prog)
{
    fprintf(stderr,
        "Usage: %s encode [options] <input|-> [-o <output|->]\n\n"
        "  Compress image(s) to .bg format.\n"
        "  Use '-' as input to read from stdin, '-' as output to write to stdout.\n\n"
        "Options:\n"
        "  -o <path>              Output file or directory\n"
        "  --quality, -q <1-100>  Encode quality (default 85)\n"
        "  --threads, -t <n>      Worker threads (default runtime)\n"
        "  --deterministic        Alias for --threads 1\n"
        "  --overwrite, -y        Overwrite existing files\n"
        "  --help                 This help\n\n"
        "Examples:\n"
        "  %s encode photo.jpg                  # → photo.bg\n"
        "  %s encode photo.jpg -o photo.bg\n"
        "  %s encode ./images -o ./out --quality 80\n"
        "  cat photo.jpg | %s encode - -o out.bg\n"
        "  cat photo.jpg | %s encode - -o -  > out.bg\n",
        prog, prog, prog, prog, prog, prog);
}

static void usage_decode(const char *prog)
{
    fprintf(stderr,
        "Usage: %s decode [options] <input.bg|-> [-o <output|->]\n\n"
        "  Decompress .bg file(s) to image.\n"
        "  Output format determined by -o extension (.jpg .png .bmp .tga .webp .pgm).\n"
        "  Use '-' as input to read from stdin, '-' as output to write to stdout (raw PGM/PPM).\n\n"
        "Options:\n"
        "  -o <path>                   Output file or directory\n"
        "  --output-quality, -Q <1-100> Output JPG/WebP quality (default 85)\n"
        "  --threads, -t <n>           Worker threads (default runtime)\n"
        "  --deterministic             Alias for --threads 1\n"
        "  --overwrite, -y             Overwrite existing files\n"
        "  --help                      This help\n\n"
        "Examples:\n"
        "  %s decode photo.bg -o photo.png\n"
        "  %s decode photo.bg                   # → photo.jpg\n"
        "  %s decode ./compressed -o ./images\n"
        "  cat photo.bg | %s decode - -o out.png\n"
        "  %s decode photo.bg -o -  | display\n",
        prog, prog, prog, prog, prog, prog);
}

static void usage_roundtrip(const char *prog)
{
    fprintf(stderr,
        "Usage: %s roundtrip [options] <input|-> [-o <output|->]\n\n"
        "  Encode + decode in memory (no .bg file written).\n"
        "  Useful for quality evaluation and pipeline testing.\n\n"
        "Options:\n"
        "  -o <path>              Output file or directory\n"
        "  --quality, -q <1-100>  Encode quality (default 85)\n"
        "  --output-quality, -Q <1-100>  Output JPG/WebP quality (default 85)\n"
        "  --threads, -t <n>      Worker threads (default runtime)\n"
        "  --deterministic        Alias for --threads 1\n"
        "  --metrics, -m          Print PSNR/SSIM after processing\n"
        "  --overwrite, -y        Overwrite existing files\n"
        "  --help                 This help\n\n"
        "Examples:\n"
        "  %s roundtrip photo.jpg -o out.jpg --quality 90 --metrics\n"
        "  %s roundtrip ./images -o ./out --quality 75\n"
        "  cat photo.jpg | %s roundtrip - -o out.jpg --metrics\n",
        prog, prog, prog, prog);
}

void cli_usage(const char *prog)
{
    fprintf(stderr,
        "bitgrain " BITGRAIN_VERSION " – image compressor (JPEG-like .bg format)\n\n"
        "Usage:\n"
        "  %s encode   [options] <input> [-o <output>]\n"
        "  %s decode   [options] <input> [-o <output>]\n"
        "  %s roundtrip [options] <input> [-o <output>]\n\n"
        "  Use '-' as input or output for stdin/stdout.\n\n"
        "Legacy flags (still supported):\n"
        "  %s -i <in> -o <out>              encode\n"
        "  %s -d -i <file.bg> -o <out>      decode\n"
        "  %s -cd -i <image> -o <out>       roundtrip\n\n"
        "Run '%s <command> --help' for command-specific options.\n",
        prog, prog, prog, prog, prog, prog, prog);
}

/* ------------------------------------------------------------------ */
/* Shared: resolve output path(s) from ctx after inputs are expanded   */
/* ------------------------------------------------------------------ */

static int resolve_output(cli_ctx_t *ctx, const char *output_path)
{
    if (ctx->use_stdout) {
        ctx->output_path = "-";
        return 0;
    }

    if (ctx->multi) {
        if (output_path) {
            int out_is_dir = 0, out_is_reg = 0;
            if (platform_stat(output_path, &out_is_dir, &out_is_reg) == 0) {
                if (out_is_dir)
                    set_output_dir(output_path);
                else {
                    fprintf(stderr, "Error: with multiple inputs -o must be a directory.\n");
                    return -1;
                }
            } else {
                if (platform_mkdir(output_path) == 0)
                    set_output_dir(output_path);
                else {
                    fprintf(stderr, "Error: could not create output directory '%s'.\n", output_path);
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
                    fprintf(stderr, "Error: '%s' already exists. Use --overwrite to overwrite.\n", ctx->output_path);
                    return -1;
                }
            }
        }
    }
    return 0;
}

/* ------------------------------------------------------------------ */
/* Subcommand parser                                                    */
/* ------------------------------------------------------------------ */

int cli_parse_subcommand(int argc, char **argv, cli_ctx_t *ctx, const char *subcmd)
{
    memset(ctx, 0, sizeof(*ctx));
    ctx->quality = 85;
    ctx->jpeg_out_quality = 85;
    ctx->threads = 0;

    if (strcmp(subcmd, "decode") == 0)
        ctx->decode_mode = 1;
    else if (strcmp(subcmd, "roundtrip") == 0)
        ctx->round_trip = 1;
    /* else: encode (default) */

    const char *output_path = NULL;
    path_list_t input_specs = { NULL, 0, 0 };

    /* argv[0] is the program name, argv[1] is the subcommand; parse from argv[2] */
    for (int i = 2; i < argc; i++) {
        const char *a = argv[i];

        if (strcmp(a, "--help") == 0 || strcmp(a, "-h") == 0) {
            if (ctx->decode_mode)       usage_decode(argv[0]);
            else if (ctx->round_trip)   usage_roundtrip(argv[0]);
            else                        usage_encode(argv[0]);
            path_list_free(&input_specs);
            return -2;
        }
        if (strcmp(a, "--version") == 0 || strcmp(a, "-v") == 0) {
            printf("bitgrain %s\n", BITGRAIN_VERSION);
            path_list_free(&input_specs);
            return -2;
        }
        /* -o / --output */
        if ((strcmp(a, "-o") == 0 || strcmp(a, "--output") == 0) && i + 1 < argc) {
            output_path = argv[++i];
            if (strcmp(output_path, "-") == 0) ctx->use_stdout = 1;
            continue;
        }

        /* --quality / -q */
        if ((strcmp(a, "--quality") == 0 || strcmp(a, "-q") == 0) && i + 1 < argc) {
            ctx->quality = atoi(argv[++i]);
            if (ctx->quality < 1)   ctx->quality = 1;
            if (ctx->quality > 100) ctx->quality = 100;
            continue;
        }
        if ((strcmp(a, "--threads") == 0 || strcmp(a, "-t") == 0) && i + 1 < argc) {
            ctx->threads = atoi(argv[++i]);
            if (ctx->threads < 1) ctx->threads = 1;
            continue;
        }
        if (strcmp(a, "--deterministic") == 0) {
            ctx->threads = 1;
            continue;
        }

        /* --output-quality / -Q */
        if ((strcmp(a, "--output-quality") == 0 || strcmp(a, "-Q") == 0) && i + 1 < argc) {
            ctx->jpeg_out_quality = atoi(argv[++i]);
            if (ctx->jpeg_out_quality < 1)   ctx->jpeg_out_quality = 1;
            if (ctx->jpeg_out_quality > 100) ctx->jpeg_out_quality = 100;
            continue;
        }

        /* --metrics / -m */
        if (strcmp(a, "--metrics") == 0 || strcmp(a, "-m") == 0) {
            ctx->show_metrics = 1;
            continue;
        }

        /* --overwrite / -y */
        if (strcmp(a, "--overwrite") == 0 || strcmp(a, "-y") == 0) {
            ctx->overwrite = 1;
            continue;
        }

        /* Positional: input path or "-" for stdin */
        if (a[0] != '-' || strcmp(a, "-") == 0) {
            if (strcmp(a, "-") == 0) {
                ctx->use_stdin = 1;
                path_list_push(&input_specs, "-");
            } else {
                path_list_push(&input_specs, a);
            }
            continue;
        }

        fprintf(stderr, "Error: unknown option '%s'. Run '%s %s --help'.\n", a, argv[0], subcmd);
        path_list_free(&input_specs);
        return -1;
    }

    if (input_specs.n == 0) {
        fprintf(stderr, "Error: missing input. Run '%s %s --help'.\n", argv[0], subcmd);
        path_list_free(&input_specs);
        return -1;
    }

    /* stdin: skip filesystem expansion */
    if (ctx->use_stdin) {
        path_list_push(&ctx->expanded, "-");
        path_list_free(&input_specs);
        ctx->multi = 0;
        if (ctx->use_stdout) {
            ctx->output_path = "-";
        } else if (output_path) {
            ctx->output_path = output_path;
        } else {
            fprintf(stderr, "Error: stdin input requires explicit -o <output>.\n");
            path_list_free(&ctx->expanded);
            return -1;
        }
        return 0;
    }

    /* Expand filesystem paths */
    int expand_bg_only = ctx->decode_mode;
    for (size_t k = 0; k < input_specs.n; k++) {
        if (path_list_append_from_spec(&ctx->expanded, input_specs.paths[k], expand_bg_only) != 0)
            fprintf(stderr, "Warning: skipping invalid or unreadable path '%s'.\n", input_specs.paths[k]);
    }
    path_list_free(&input_specs);

    if (ctx->expanded.n == 0) {
        fprintf(stderr, "Error: no %s found in the given path(s).\n",
                ctx->decode_mode ? ".bg files" : "image files");
        return -1;
    }

    ctx->multi = (ctx->expanded.n > 1);

    if (resolve_output(ctx, output_path) != 0) {
        path_list_free(&ctx->expanded);
        return -1;
    }

    return 0;
}

/* ------------------------------------------------------------------ */
/* Legacy flag-based parser (backward compat)                          */
/* ------------------------------------------------------------------ */

int cli_parse(int argc, char **argv, cli_ctx_t *ctx)
{
    const char *input_path = NULL;
    const char *output_path = NULL;
    int opt;

    memset(ctx, 0, sizeof(*ctx));
    ctx->quality = 85;
    ctx->jpeg_out_quality = 85;
    ctx->threads = 0;

    while ((opt = getopt(argc, argv, "i:o:cdq:Q:t:myvh")) != -1) {
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
        case 't':
            ctx->threads = atoi(optarg);
            if (ctx->threads < 1) ctx->threads = 1;
            break;
        case 'm': ctx->show_metrics = 1; break;
        case 'y': ctx->overwrite = 1; break;
        case 'v':
            printf("bitgrain %s\n", BITGRAIN_VERSION);
            printf("Author: Iván E. Rodriguez\n");
            printf("License: GPLv3\n");
            printf("Upstream: https://github.com/IRodriguez13/bitgrain\n");
            return -2;
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

    if (output_path && strcmp(output_path, "-") == 0) ctx->use_stdout = 1;

    if (resolve_output(ctx, output_path) != 0) {
        path_list_free(&ctx->expanded);
        return -1;
    }

    return 0;
}

void cli_ctx_free(cli_ctx_t *ctx)
{
    path_list_free(&ctx->expanded);
    free(ctx->output_path_owned);
    free(ctx->output_dir_owned);
}
