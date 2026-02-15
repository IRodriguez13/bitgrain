/* SPDX-License-Identifier: GPL-3.0-or-later */
#define _POSIX_C_SOURCE 200809L
#include "path_utils.h"
#include "platform.h"
#include <stdlib.h>
#include <string.h>
#include <stdio.h>

static const char *const IMAGE_EXTS[] = {
    ".jpg", ".jpeg", ".png", ".bmp", ".gif", ".tga", ".pgm", ".psd", ".hdr", ".webp"
};
static const size_t NUM_IMAGE_EXTS = sizeof(IMAGE_EXTS) / sizeof(IMAGE_EXTS[0]);

int is_image_extension(const char *path)
{
    const char *dot = strrchr(path, '.');
    if (!dot || dot == path) return 0;
    size_t ext_len = strlen(dot);
    for (size_t i = 0; i < NUM_IMAGE_EXTS; i++) {
        if (ext_len != strlen(IMAGE_EXTS[i])) continue;
        if (platform_strcasecmp(dot, IMAGE_EXTS[i]) == 0) return 1;
    }
    return 0;
}

static int is_bg_extension(const char *path)
{
    const char *dot = strrchr(path, '.');
    return (dot && dot != path && platform_strcasecmp(dot, ".bg") == 0);
}

void path_list_push(path_list_t *list, const char *path)
{
    if (list->n >= list->cap) {
        size_t new_cap = list->cap ? list->cap * 2 : 32;
        char **p = (char **)realloc(list->paths, new_cap * sizeof(char *));
        if (!p) return;
        list->paths = p;
        list->cap = new_cap;
    }
    list->paths[list->n++] = strdup(path);
}

void path_list_free(path_list_t *list)
{
    if (!list) return;
    for (size_t i = 0; i < list->n; i++) free(list->paths[i]);
    free(list->paths);
    list->paths = NULL;
    list->n = list->cap = 0;
}

int path_list_append_from_spec(path_list_t *list, const char *spec, int bg_only)
{
    int is_dir = 0, is_reg = 0;
    if (platform_stat(spec, &is_dir, &is_reg) != 0) return -1;
    if (is_reg) {
        if (bg_only ? is_bg_extension(spec) : is_image_extension(spec))
            path_list_push(list, spec);
        return 0;
    }
    if (is_dir) {
        platform_dir_t *d = platform_dir_open(spec);
        if (!d) return -1;
        size_t spec_len = strlen(spec);
        int need_slash = (spec_len > 0 && spec[spec_len - 1] != '/' && spec[spec_len - 1] != '\\');
        char *prefix = (char *)malloc(spec_len + (need_slash ? 2 : 1));
        if (!prefix) { platform_dir_close(d); return -1; }
        memcpy(prefix, spec, spec_len + 1);
        if (need_slash) { prefix[spec_len] = '/'; prefix[spec_len + 1] = '\0'; }
        char namebuf[256];
        while (platform_dir_next(d, namebuf, sizeof(namebuf)) == 1) {
            if (namebuf[0] == '.') continue;
            int add = bg_only ? is_bg_extension(namebuf) : is_image_extension(namebuf);
            if (!add) continue;
            size_t full_len = strlen(prefix) + strlen(namebuf) + 1;
            char *full = (char *)malloc(full_len);
            if (!full) continue;
            snprintf(full, full_len, "%s%s", prefix, namebuf);
            path_list_push(list, full);
            free(full);
        }
        free(prefix);
        platform_dir_close(d);
        return 0;
    }
    return -1;
}

static const char *get_ext(const char *path)
{
    const char *dot = strrchr(path, '.');
    if (!dot || dot == path) return "";
    return dot;
}

int default_output_path(const char *input, char *out_buf, size_t buf_size, int decode_mode, int round_trip)
{
    const char *ext = get_ext(input);
    size_t base_len = ext ? (size_t)(ext - input) : strlen(input);
    size_t ext_len = (decode_mode || round_trip) ? 5u : 4u;

    if (decode_mode || round_trip) {
        if (base_len + ext_len >= buf_size) return -1;
        memcpy(out_buf, input, base_len);
        memcpy(out_buf + base_len, ".jpg", ext_len);
        if (strcmp(out_buf, input) == 0) {
            if (base_len + 9 >= buf_size) return -1;
            memcpy(out_buf + base_len, " (1).jpg", 8);
            out_buf[base_len + 8] = '\0';
        }
    } else {
        if (base_len + ext_len >= buf_size) return -1;
        memcpy(out_buf, input, base_len);
        memcpy(out_buf + base_len, ".bg", ext_len);
    }
    return 0;
}

char *avoid_overwrite_path(const char *path)
{
    const char *ext = get_ext(path);
    size_t base_len = ext ? (size_t)(ext - path) : strlen(path);
    size_t slen = base_len + 32;
    char *out = (char *)malloc(slen);
    if (!out) return NULL;

    memcpy(out, path, base_len);
    out[base_len] = '\0';
    for (int n = 1; n <= 9999; n++) {
        int len = snprintf(out + base_len, slen - base_len, " (%d)%s", n, ext ? ext : "");
        if (len < 0 || (size_t)len >= slen - base_len) break;
        FILE *f = fopen(out, "rb");
        if (!f) return out;
        fclose(f);
    }
    free(out);
    return NULL;
}
