/* SPDX-License-Identifier: GPL-3.0-or-later  Copyright (C) 2026 Iván E. Rodriguez */

#define _POSIX_C_SOURCE 200809L

#include <stdio.h>
#include <stdlib.h>

#include "cli.h"
#include "roundtrip_cli.h"
#include "decode_cli.h"
#include "encode_cli.h"

int main(int argc, char **argv)
{
    cli_ctx_t ctx;
    int r = cli_parse(argc, argv, &ctx);
    if (r == -2) return 0;  /* -v or -h */
    if (r != 0) return 1;

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
