/* SPDX-License-Identifier: GPL-3.0-or-later */
#ifndef BITGRAIN_PATH_UTILS_H
#define BITGRAIN_PATH_UTILS_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Dynamic list of path strings. */
typedef struct {
    char **paths;
    size_t n;
    size_t cap;
} path_list_t;

void path_list_push(path_list_t *list, const char *path);
void path_list_free(path_list_t *list);

int is_image_extension(const char *path);

/* Append paths from spec (file or directory). If bg_only, only .bg; else only image exts. Returns 0 on success. */
int path_list_append_from_spec(path_list_t *list, const char *spec, int bg_only);

/* Default output path. Returns 0 on success. */
int default_output_path(const char *input, char *out_buf, size_t buf_size, int decode_mode, int round_trip);

/* If path exists, return new path "base (n).ext" that does not exist. Caller frees. NULL if not needed. */
char *avoid_overwrite_path(const char *path);

#ifdef __cplusplus
}
#endif

#endif
