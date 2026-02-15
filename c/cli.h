/* SPDX-License-Identifier: GPL-3.0-or-later */
#ifndef BITGRAIN_CLI_H
#define BITGRAIN_CLI_H

#include "path_utils.h"
#include <stdint.h>

#define BITGRAIN_VERSION "1.0.0"

typedef struct {
    path_list_t expanded;
    const char *output_path;   /* single output file */
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
} cli_ctx_t;

void cli_usage(const char *prog);

/* Parse args, fill ctx. Returns 0 on success, -1 on error (usage printed). */
int cli_parse(int argc, char **argv, cli_ctx_t *ctx);

/* Free owned fields in ctx. */
void cli_ctx_free(cli_ctx_t *ctx);

#endif
