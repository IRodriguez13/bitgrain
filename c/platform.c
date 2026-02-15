/* SPDX-License-Identifier: GPL-3.0-or-later
 * Platform abstraction: directories, stat, strcasecmp, getopt (Windows).
 */

#include "platform.h"
#include <stdlib.h>
#include <string.h>

#ifdef _WIN32
#include <windows.h>
#include <direct.h>
#include <sys/stat.h>
#include <io.h>

int platform_mkdir(const char *path)
{
    return _mkdir(path);
}

int platform_stat(const char *path, int *is_dir, int *is_reg)
{
    struct _stat st;
    if (_stat(path, &st) != 0) return -1;
    if (is_dir) *is_dir = ((st.st_mode & _S_IFMT) == _S_IFDIR) ? 1 : 0;
    if (is_reg) *is_reg = ((st.st_mode & _S_IFMT) == _S_IFREG) ? 1 : 0;
    return 0;
}

int platform_strcasecmp(const char *a, const char *b)
{
    return _stricmp(a, b);
}

struct platform_dir {
    HANDLE find;
    WIN32_FIND_DATAA data;
    int first;
};

platform_dir_t *platform_dir_open(const char *path)
{
    char pattern[MAX_PATH];
    size_t len = strlen(path);
    if (len >= MAX_PATH - 3) return NULL;
    memcpy(pattern, path, len + 1);
    if (len > 0 && (path[len-1] != '\\' && path[len-1] != '/')) {
        pattern[len] = '\\';
        pattern[len+1] = '*';
        pattern[len+2] = '\0';
    } else {
        pattern[len] = '*';
        pattern[len+1] = '\0';
    }
    platform_dir_t *d = (platform_dir_t *)malloc(sizeof(platform_dir_t));
    if (!d) return NULL;
    d->find = FindFirstFileA(pattern, &d->data);
    d->first = 1;
    if (d->find == INVALID_HANDLE_VALUE) {
        free(d);
        return NULL;
    }
    return d;
}

int platform_dir_next(platform_dir_t *d, char *namebuf, size_t bufsize)
{
    if (!d || !namebuf || bufsize == 0) return -1;
    for (;;) {
        if (d->first) {
            d->first = 0;
        } else {
            if (!FindNextFileA(d->find, &d->data)) return 0;
        }
        if (strcmp(d->data.cFileName, ".") == 0 || strcmp(d->data.cFileName, "..") == 0)
            continue;
        size_t n = strlen(d->data.cFileName);
        if (n >= bufsize) return -1;
        memcpy(namebuf, d->data.cFileName, n + 1);
        return 1;
    }
}

void platform_dir_close(platform_dir_t *d)
{
    if (d) {
        if (d->find != INVALID_HANDLE_VALUE) FindClose(d->find);
        free(d);
    }
}

/* Minimal getopt for Windows (POSIX-like). */
char *optarg = NULL;
int optind = 1, opterr = 1, optopt;
static char *platform_place = "";

int getopt(int argc, char *const argv[], const char *optstring)
{
    char *oli;
    if (!*platform_place) {
        if (optind >= argc || argv[optind][0] != '-' || argv[optind][1] == '\0')
            return -1;
        if (strcmp(argv[optind], "--") == 0) {
            optind++;
            return -1;
        }
        platform_place = (char *)argv[optind] + 1;
    }
    optopt = (unsigned char)*platform_place++;
    if (optopt == ':' || (oli = strchr(optstring, optopt)) == NULL) {
        if (*platform_place == '\0') optind++;
        if (opterr) (void)0; /* could fprintf to stderr */
        return '?';
    }
    if (oli[1] == ':') {
        if (*platform_place) {
            optarg = platform_place;
            platform_place = "";
        } else if (argc <= ++optind) {
            platform_place = "";
            if (opterr) (void)0;
            return '?';
        } else
            optarg = argv[optind];
        optind++;
    }
    if (*platform_place == '\0') platform_place = "", optind++;
    return optopt;
}

#else
/* POSIX (Linux, macOS, BSD, etc.) */
#include <dirent.h>
#include <strings.h>
#include <sys/stat.h>

int platform_mkdir(const char *path)
{
    return mkdir(path, 0755);
}

int platform_stat(const char *path, int *is_dir, int *is_reg)
{
    struct stat st;
    if (stat(path, &st) != 0) return -1;
    if (is_dir) *is_dir = S_ISDIR(st.st_mode) ? 1 : 0;
    if (is_reg) *is_reg = S_ISREG(st.st_mode) ? 1 : 0;
    return 0;
}

int platform_strcasecmp(const char *a, const char *b)
{
    return strcasecmp(a, b);
}

struct platform_dir {
    DIR *dir;
};

platform_dir_t *platform_dir_open(const char *path)
{
    DIR *dir = opendir(path);
    if (!dir) return NULL;
    platform_dir_t *d = (platform_dir_t *)malloc(sizeof(platform_dir_t));
    if (!d) { closedir(dir); return NULL; }
    d->dir = dir;
    return d;
}

int platform_dir_next(platform_dir_t *d, char *namebuf, size_t bufsize)
{
    if (!d || !d->dir || !namebuf || bufsize == 0) return -1;
    struct dirent *ent = readdir(d->dir);
    if (!ent) return 0;
    size_t n = strlen(ent->d_name);
    if (n >= bufsize) return -1;
    memcpy(namebuf, ent->d_name, n + 1);
    return 1;
}

void platform_dir_close(platform_dir_t *d)
{
    if (d) {
        if (d->dir) closedir(d->dir);
        free(d);
    }
}
#endif
