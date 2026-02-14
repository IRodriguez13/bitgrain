/* SPDX-License-Identifier: GPL-3.0-or-later  Copyright (C) 2026 Iván E. Rodriguez */

#define _POSIX_C_SOURCE 200809L

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include <unistd.h>

#include "encoder.h"
#include "image_loader.h"

#define OUT_BUF_SIZE (1024 * 1024 * 4)
#define MAX_PIXELS   (4096 * 4096)

static void usage(const char *prog)
{
    fprintf(stderr,
            "bitgrain – comprimir y descomprimir imágenes (encode ↔ decode)\n\n"
            "Comprimir (imagen → .bg):\n"
            "  %s -i <imagen> -o <salida.bg>\n"
            "  %s <imagen>                    → <imagen>.bg\n\n"
            "Descomprimir (.bg → imagen):\n"
            "  %s -d -i <archivo.bg> -o <imagen.pgm>\n"
            "  %s -d <archivo.bg>             → <archivo>.pgm\n\n"
            "Opciones:\n"
            "  -i <archivo>   entrada (imagen o .bg según modo)\n"
            "  -o <archivo>   salida\n"
            "  -d             descomprimir (.bg → PGM)\n"
            "  -y             sobrescribir salida\n"
            "  -h             esta ayuda\n",
            prog, prog, prog, prog);
}

/* Devuelve extensión (incluyendo el punto) o "" si no hay. */
static const char *get_ext(const char *path)
{
    const char *dot = strrchr(path, '.');
    if (!dot || dot == path) return "";
    return dot;
}

/* Salida por defecto: encode → .bg, decode → .pgm */
static int default_output_path(const char *input, char *out_buf, size_t buf_size, int decode_mode)
{
    const char *ext = get_ext(input);
    size_t base_len = ext ? (size_t)(ext - input) : strlen(input);
    if (decode_mode) {
        if (base_len + 5 >= buf_size) return -1;
        memcpy(out_buf, input, base_len);
        memcpy(out_buf + base_len, ".pgm", 5);
    } else {
        if (base_len + 4 >= buf_size) return -1;
        memcpy(out_buf, input, base_len);
        memcpy(out_buf + base_len, ".bg", 4);
    }
    return 0;
}

static int write_pgm(const char *path, const uint8_t *pixels, uint32_t width, uint32_t height)
{
    FILE *f = fopen(path, "wb");
    if (!f) return -1;
    fprintf(f, "P5\n%u %u\n255\n", width, height);
    size_t n = (size_t)width * (size_t)height;
    if (fwrite(pixels, 1, n, f) != n) {
        fclose(f);
        return -1;
    }
    fclose(f);
    return 0;
}

int main(int argc, char **argv)
{
    const char *input_path  = NULL;
    const char *output_path = NULL;
    char *output_path_owned = NULL;
    int overwrite = 0;
    int decode_mode = 0;
    int opt;

    while ((opt = getopt(argc, argv, "i:o:dyh")) != -1) {
        switch (opt) {
        case 'i':
            input_path = optarg;
            break;
        case 'o':
            output_path = optarg;
            break;
        case 'd':
            decode_mode = 1;
            break;
        case 'y':
            overwrite = 1;
            break;
        case 'h':
            usage(argv[0]);
            return 0;
        default:
            usage(argv[0]);
            return 1;
        }
    }

    if (!input_path && optind < argc) {
        input_path = argv[optind];
    }

    if (!input_path) {
        fprintf(stderr, "Error: falta archivo de entrada.\n");
        usage(argv[0]);
        return 1;
    }

    if (!output_path) {
        char def[1024];
        if (default_output_path(input_path, def, sizeof(def), decode_mode) != 0) {
            fprintf(stderr, "Error: ruta de entrada demasiado larga.\n");
            return 1;
        }
        output_path_owned = strdup(def);
        if (!output_path_owned) {
            fprintf(stderr, "Error: sin memoria.\n");
            return 1;
        }
        output_path = output_path_owned;
    }

    if (!overwrite) {
        FILE *exists = fopen(output_path, "rb");
        if (exists) {
            fclose(exists);
            fprintf(stderr, "Error: '%s' ya existe. Usa -y para sobrescribir.\n", output_path);
            free(output_path_owned);
            return 1;
        }
    }

    if (decode_mode) {
        /* Descomprimir .bg → PGM */
        FILE *f = fopen(input_path, "rb");
        if (!f) {
            fprintf(stderr, "Error: no se pudo abrir '%s'.\n", input_path);
            free(output_path_owned);
            return 1;
        }
        if (fseek(f, 0, SEEK_END) != 0) {
            fclose(f);
            free(output_path_owned);
            return 1;
        }
        long fsize = ftell(f);
        if (fsize <= 0 || fsize > (long)OUT_BUF_SIZE) {
            fclose(f);
            free(output_path_owned);
            fprintf(stderr, "Error: archivo .bg inválido o demasiado grande.\n");
            return 1;
        }
        rewind(f);
        uint8_t *bg_buf = (uint8_t *)malloc((size_t)fsize);
        if (!bg_buf) {
            fclose(f);
            free(output_path_owned);
            return 1;
        }
        if (fread(bg_buf, 1, (size_t)fsize, f) != (size_t)fsize) {
            fclose(f);
            free(bg_buf);
            free(output_path_owned);
            return 1;
        }
        fclose(f);

        uint8_t *pixels = (uint8_t *)malloc(MAX_PIXELS);
        if (!pixels) {
            free(bg_buf);
            free(output_path_owned);
            return 1;
        }
        uint32_t width, height;
        int ret = bitgrain_decode_grayscale(bg_buf, (int32_t)fsize, pixels, MAX_PIXELS, &width, &height);
        free(bg_buf);
        if (ret != 0) {
            fprintf(stderr, "Error: '%s' no es un .bg válido o está corrupto.\n", input_path);
            free(pixels);
            free(output_path_owned);
            return 1;
        }
        if (write_pgm(output_path, pixels, width, height) != 0) {
            fprintf(stderr, "Error: no se pudo escribir '%s'.\n", output_path);
            free(pixels);
            free(output_path_owned);
            return 1;
        }
        free(pixels);
        printf("%s -> %s  (%u×%u)\n", input_path, output_path, width, height);
        free(output_path_owned);
        return 0;
    }

    /* Comprimir imagen → .bg */
    uint32_t width, height;
    uint8_t *pixels = bitgrain_load_grayscale(input_path, &width, &height);
    if (!pixels) {
        fprintf(stderr, "Error: no se pudo cargar '%s' (¿archivo existe y es una imagen válida?).\n", input_path);
        free(output_path_owned);
        return 1;
    }

    uint8_t *out_buf = (uint8_t *)malloc(OUT_BUF_SIZE);
    if (!out_buf) {
        bitgrain_image_free(pixels);
        free(output_path_owned);
        return 1;
    }

    int32_t out_len = 0;
    int ret = bitgrain_encode_grayscale(pixels, width, height,
                                         out_buf, OUT_BUF_SIZE, &out_len);
    bitgrain_image_free(pixels);

    if (ret != 0) {
        fprintf(stderr, "Error en el encoder.\n");
        free(out_buf);
        free(output_path_owned);
        return 1;
    }

    FILE *out = fopen(output_path, "wb");
    if (!out) {
        fprintf(stderr, "Error: no se pudo crear '%s'.\n", output_path);
        free(out_buf);
        free(output_path_owned);
        return 1;
    }
    if (fwrite(out_buf, 1, (size_t)out_len, out) != (size_t)out_len) {
        fprintf(stderr, "Error al escribir la salida.\n");
        fclose(out);
        free(out_buf);
        free(output_path_owned);
        return 1;
    }
    fclose(out);
    free(out_buf);

    printf("%s -> %s  (%u×%u, %d bytes)\n",
           input_path, output_path, width, height, (int)out_len);
    free(output_path_owned);
    return 0;
}
