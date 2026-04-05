/* SPDX-License-Identifier: GPL-3.0-or-later */
#ifndef BITGRAIN_CLI_H
#define BITGRAIN_CLI_H

#include "path_utils.h"
#include <stdint.h>

#define BITGRAIN_VERSION "1.1.0"

typedef struct {
    path_list_t expanded;
    const char *output_path;   /* single output file or "-" for stdout */
    char *output_path_owned;
    const char *output_dir;    /* for multi; may point to static or owned */
    char *output_dir_owned;
    int multi;
    int overwrite;
    int decode_mode;
    int round_trip;
    int quality;
    int jpeg_out_quality;
    int show_metrics;
    int threads;               /* worker threads; 0 = runtime default */
    int use_stdin;             /* input is stdin ("-") */
    int use_stdout;            /* output is stdout ("-") */
} cli_ctx_t;

void cli_usage(const char *prog);

/* Legacy flag-based parse (backward compat). Returns 0 ok, -1 error, -2 exit-0. */
int cli_parse(int argc, char **argv, cli_ctx_t *ctx);

/* Subcommand parse: argv[0] is the program name, argv[1] is the subcommand.
   Returns 0 ok, -1 error, -2 exit-0. */
int cli_parse_subcommand(int argc, char **argv, cli_ctx_t *ctx, const char *subcmd);

/* Free owned fields in ctx. */
void cli_ctx_free(cli_ctx_t *ctx);

#endif
