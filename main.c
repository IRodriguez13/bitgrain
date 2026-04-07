/* SPDX-License-Identifier: GPL-3.0-or-later  Copyright (C) 2026 Iván E. Rodriguez */

#define _POSIX_C_SOURCE 200809L

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "cli.h"
#include "encoder.h"
#include "roundtrip_cli.h"
#include "decode_cli.h"
#include "encode_cli.h"

static void print_global_help(const char *prog)
{
    fprintf(stderr,
        "bitgrain " BITGRAIN_VERSION " – image compressor (.bg format)\n\n"
        "Usage:\n"
        "  %s encode   [options] <input> [-o <output>]\n"
        "  %s decode   [options] <input> [-o <output>]\n"
        "  %s roundtrip [options] <input> [-o <output>]\n\n"
        "  Use '-' as input or output for stdin/stdout.\n\n"
        "Commands:\n"
        "  encode     Compress image(s) to .bg format\n"
        "  decode     Decompress .bg file(s) to image\n"
        "  roundtrip  Encode + decode in memory (no .bg written)\n\n"
        "Options (all commands):\n"
        "  -o <path>            Output file or directory\n"
        "  --quality <1-100>    Encode quality (default 85)\n"
        "  --output-quality <1-100>  Output JPG/WebP quality (default 85)\n"
        "  --threads <n>        Worker threads (default runtime)\n"
        "  --deterministic      Alias for --threads 1\n"
        "  --overwrite          Overwrite existing files\n"
        "  --metrics            Print PSNR/SSIM (roundtrip only)\n"
        "  --help               Show this help\n"
        "  --version            Show version\n\n"
        "Short flags (legacy):\n"
        "  -q <1-100>   encode quality   -Q <1-100>  output JPG quality\n"
        "  -y           overwrite        -m          metrics\n\n"
        "Examples:\n"
        "  %s encode photo.jpg -o photo.bg\n"
        "  %s encode photo.jpg              # → photo.bg\n"
        "  %s decode photo.bg -o photo.png\n"
        "  %s roundtrip photo.jpg -o out.jpg --quality 90 --metrics\n"
        "  cat photo.jpg | %s encode - -o out.bg\n"
        "  %s decode photo.bg -o -  | display\n"
        "  %s encode ./images -o ./compressed --quality 80\n",
        prog, prog, prog, prog, prog, prog, prog, prog, prog, prog);
}

/* Detect if argv[1] is a known subcommand. */
static int is_subcommand(const char *s)
{
    if (!s) return 0;
    return (strcmp(s, "encode") == 0 ||
            strcmp(s, "decode") == 0 ||
            strcmp(s, "roundtrip") == 0);
}

int main(int argc, char **argv)
{
    /* Global --help / --version before subcommand */
    if (argc >= 2 && (strcmp(argv[1], "--help") == 0 || strcmp(argv[1], "-h") == 0)) {
        print_global_help(argv[0]);
        return 0;
    }
    if (argc >= 2 && (strcmp(argv[1], "--version") == 0 || strcmp(argv[1], "-v") == 0)) {
        printf("bitgrain %s\n", BITGRAIN_VERSION);
        printf("Author: Iván E. Rodriguez <ivanrwcm25@gmail.com>\n");
        printf("License GPLv3+: GNU GPL version 3 or later <http://gnu.org/licenses/gpl.html>\n");
        printf("This is free software: you are free to redistribute it and/or modify it.\n");
        printf("There is NO WARRANTY, to the extent permitted by applicable law.\n");
        printf("Upstream: https://github.com/IRodriguez13/bitgrain\n");
        return 0;
    }

    cli_ctx_t ctx;
    int r;

    if (argc >= 2 && is_subcommand(argv[1])) {
        /* New subcommand mode: pass full argv so argv[0] stays as program name */
        const char *subcmd = argv[1];
        r = cli_parse_subcommand(argc, argv, &ctx, subcmd);
        if (r == -2) return 0;
        if (r != 0) return 1;
    } else {
        /* Legacy flag-based mode for backward compatibility */
        r = cli_parse(argc, argv, &ctx);
        if (r == -2) return 0;
        if (r != 0) return 1;
    }

    if (ctx.threads > 0) {
        if (bitgrain_set_threads(ctx.threads) != 0) {
            fprintf(stderr, "Error: could not set worker threads to %d (set before codec use).\n", ctx.threads);
            cli_ctx_free(&ctx);
            return 1;
        }
    }

    int ret;
    if (ctx.round_trip)
        ret = roundtrip_cli_run(&ctx);
    else if (ctx.decode_mode)
        ret = decode_cli_run(&ctx);
    else
        ret = encode_cli_run(&ctx);

    cli_ctx_free(&ctx);
    return ret;
}
