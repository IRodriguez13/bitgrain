/* SPDX-License-Identifier: GPL-3.0-or-later */
#ifndef BITGRAIN_ENCODE_CLI_H
#define BITGRAIN_ENCODE_CLI_H

#include "cli.h"

/* Run encode for all files in ctx. Returns 0 on success, 1 if any failed. */
int encode_cli_run(const cli_ctx_t *ctx);

#endif
