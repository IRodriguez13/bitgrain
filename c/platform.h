#ifndef BITGRAIN_PLATFORM_H
#define BITGRAIN_PLATFORM_H

#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Create directory (parent must exist). Returns 0 on success, -1 on error. */
int platform_mkdir(const char *path);

/* Portable stat: fill is_dir and is_reg (1 = true, 0 = false). Returns 0 on success, -1 if path missing/invalid. */
int platform_stat(const char *path, int *is_dir, int *is_reg);

/* Case-insensitive string compare. Returns <0, 0, >0 like strcmp. */
int platform_strcasecmp(const char *a, const char *b);

/* Directory iteration (opaque). */
typedef struct platform_dir platform_dir_t;

platform_dir_t *platform_dir_open(const char *path);
/* Next entry: writes current name into namebuf (max bufsize). Returns 1 if name written, 0 at end, -1 on error. */
int platform_dir_next(platform_dir_t *d, char *namebuf, size_t bufsize);
void platform_dir_close(platform_dir_t *d);

/* getopt: on Windows we provide our own; on POSIX use system. */
#ifdef _WIN32
extern int getopt(int argc, char *const argv[], const char *optstring);
extern char *optarg;
extern int optind, opterr, optopt;
#else
#include <unistd.h>
#endif

#ifdef __cplusplus
}
#endif

#endif
