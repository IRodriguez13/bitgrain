/* SPDX-License-Identifier: GPL-3.0-or-later */
#ifndef BITGRAIN_ROUNDTRIP_CLI_H
#define BITGRAIN_ROUNDTRIP_CLI_H

#include "cli.h"

/* Run round-trip for all files in ctx. Returns 0 on success, 1 if any failed. */
int roundtrip_cli_run(const cli_ctx_t *ctx);

#endif
