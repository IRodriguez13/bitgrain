/* SPDX-License-Identifier: GPL-3.0-or-later */
#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include <string.h>
#include "encoder.h"

static uint8_t *read_file(const char *path, size_t *out_len) {
    FILE *f = fopen(path, "rb");
    if (!f) return NULL;
    if (fseek(f, 0, SEEK_END) != 0) { fclose(f); return NULL; }
    long n = ftell(f);
    if (n <= 0) { fclose(f); return NULL; }
    rewind(f);
    uint8_t *buf = (uint8_t *)malloc((size_t)n);
    if (!buf) { fclose(f); return NULL; }
    if (fread(buf, 1, (size_t)n, f) != (size_t)n) { fclose(f); free(buf); return NULL; }
    fclose(f);
    *out_len = (size_t)n;
    return buf;
}

static int header_channels(const uint8_t *bg) {
    if (!bg || bg[0] != 'B' || bg[1] != 'G') return 0;
    switch (bg[2]) {
        case 1: return 1;
        case 2: return 3;
        case 3: return 4;
        case 4: return 3;
        case 5: return 4;
        default: return 0;
    }
}

int main(int argc, char **argv) {
    if (argc < 2) {
        fprintf(stderr, "usage: %s <input.bg>\n", argv[0]);
        return 2;
    }

    bitgrain_clear_error();
    if (bitgrain_decode(NULL, 0, NULL, 0, NULL, NULL, NULL) != -1) {
        fprintf(stderr, "expected decode(NULL) to fail\n");
        return 3;
    }
    fprintf(stdout, "null decode error: code=%d msg=%s\n",
            bitgrain_last_error_code(), bitgrain_last_error_message());

    if (bitgrain_set_threads(1) != 0) {
        fprintf(stderr, "bitgrain_set_threads failed: code=%d msg=%s\n",
                bitgrain_last_error_code(), bitgrain_last_error_message());
        return 4;
    }

    size_t bg_len = 0;
    uint8_t *bg = read_file(argv[1], &bg_len);
    if (!bg) {
        fprintf(stderr, "could not read %s\n", argv[1]);
        return 5;
    }

    if (bg_len < 11 || bg[0] != 'B' || bg[1] != 'G') {
        fprintf(stderr, "invalid .bg header\n");
        free(bg);
        return 6;
    }

    uint32_t w = (uint32_t)bg[3] | ((uint32_t)bg[4] << 8) | ((uint32_t)bg[5] << 16) | ((uint32_t)bg[6] << 24);
    uint32_t h = (uint32_t)bg[7] | ((uint32_t)bg[8] << 8) | ((uint32_t)bg[9] << 16) | ((uint32_t)bg[10] << 24);
    int ch_guess = header_channels(bg);
    if (w == 0 || h == 0 || ch_guess == 0) {
        fprintf(stderr, "invalid width/height/channels in header\n");
        free(bg);
        return 7;
    }

    size_t cap = (size_t)w * (size_t)h * (size_t)ch_guess;
    uint8_t *out = (uint8_t *)malloc(cap);
    if (!out) {
        free(bg);
        return 8;
    }

    uint32_t dw = 0, dh = 0, dc = 0;
    int ret = bitgrain_decode(bg, (int32_t)bg_len, out, (uint32_t)cap, &dw, &dh, &dc);
    if (ret != 0) {
        fprintf(stderr, "decode failed: code=%d msg=%s\n",
                bitgrain_last_error_code(), bitgrain_last_error_message());
        free(out);
        free(bg);
        return 9;
    }

    fprintf(stdout, "decode ok: %ux%u ch=%u bytes=%zu\n", dw, dh, dc, cap);
    bitgrain_clear_error();
    fprintf(stdout, "after clear: code=%d msg=%s\n",
            bitgrain_last_error_code(), bitgrain_last_error_message());

    free(out);
    free(bg);
    return 0;
}
