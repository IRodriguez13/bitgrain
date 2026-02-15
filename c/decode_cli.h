/* SPDX-License-Identifier: GPL-3.0-or-later */
#ifndef BITGRAIN_DECODE_CLI_H
#define BITGRAIN_DECODE_CLI_H

#include "cli.h"

/* Run decode for all files in ctx. Returns 0 on success, 1 if any failed. */
int decode_cli_run(const cli_ctx_t *ctx);

#endif
