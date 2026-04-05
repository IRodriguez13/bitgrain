# Bitgrain

Image compressor (JPEG-like). Encodes to a custom `.bg` stream; decodes to pixels or standard image files. Grayscale, RGB, RGBA. CLI + C API (FFI-backed) with deterministic mode support.

## Build

Requirements: Rust (stable), GCC (C11), make, **libwebp** (headers + lib).

```bash
./setup.sh
```

El script instala dependencias si faltan. Manual:

| Sistema | Comando |
|---------|---------|
| Debian/Ubuntu | `sudo apt install libwebp-dev` |
| Fedora/RHEL | `sudo dnf install libwebp-devel` |
| macOS | `brew install webp` |

Luego:

```bash
make bitgrain
```

Install: `sudo make install` (or `PREFIX=$HOME/.local make install`). Installs `bitgrain`, `include/bitgrain/encoder.h`, `lib/libbitgrain.a`.

Portable build: `make CFLAGS_NATIVE= RUSTFLAGS_NATIVE= bitgrain`.

## CLI

Preferred command style uses subcommands:

```bash
bitgrain encode <input> [-o output.bg] [--quality 1-100]
bitgrain decode <input.bg> [-o output.{png,jpg,webp,bmp,tga,pgm}]
bitgrain roundtrip <input> [-o output.jpg] [--quality 1-100] [--metrics]
```

Legacy flags are still supported (`-i/-d/-cd/...`) for backward compatibility.

| Option | Description |
|--------|-------------|
| `-o, --output <path>` | Output file or directory |
| `-q, --quality <1-100>` | Encode quality (default 85) |
| `-Q, --output-quality <1-100>` | Output JPG/WebP quality (default 85) |
| `-t, --threads <n>` | Worker threads for codec internals |
| `--deterministic` | Alias for `--threads 1` |
| `-m, --metrics` | Round-trip: print PSNR and SSIM |
| `-y, --overwrite` | Overwrite outputs |
| `-v, --version` / `-h, --help` | Version / help |

Input formats: JPEG, PNG, BMP, GIF, TGA, PGM, PSD, HDR, WebP (stb_image + libwebp).

Decode/roundtrip output formats by `-o` extension: `.jpg/.jpeg`, `.png`, `.bmp`, `.tga`, `.pgm` (grayscale), `.webp`.

## Format .bg

Header (12 bytes): magic `"BG"`, version, width (u32 LE), height (u32 LE), quality.

Versions:

- `v1`: grayscale + RLE entropy
- `v2`: RGB planar + RLE entropy (legacy)
- `v3`: RGBA planar + RLE entropy (legacy)
- `v4`: YCbCr 4:2:0 + Huffman (RGB output)
- `v5`: YCbCr 4:2:0 + alpha + Huffman (RGBA output)

## C API

`includes/encoder.h`.

- Encode: `bitgrain_encode_grayscale`, `bitgrain_encode_rgb`, `bitgrain_encode_rgba`
- Decode: `bitgrain_decode(buf, size, pixels, cap, &w, &h, &channels)`
- Threading: `bitgrain_set_threads`
- Error state: `bitgrain_last_error_code`, `bitgrain_last_error_message`, `bitgrain_clear_error`

Channels are `1`, `3`, or `4`. Link `libbitgrain.a` and C objects; add `-lpthread -ldl -lm -lwebp`.

## Prioridades

- **Optimización encode/decode:** Cuantización con SIMD (SSE2/NEON) en C; DCT/IDCT en referencia. Objetivo: ser más rápido que WebP/AVIF con calidad similar.
- **Transparencia y canales alfa:** **Hecho.** PNG y WebP con alpha se cargan como RGBA; .bg versión 3 (4 canales); decode devuelve 1, 3 o 4 canales según cabecera.

## Integración

- **Crate Rust:** `rust/Cargo.toml` listo para publicar en crates.io (description, license, repository, keywords). Publicar con `cargo publish` desde `rust/`.
- **Librería C:** `make install` instala estática `lib/libbitgrain.a` y cabeceras. Opcional: se genera también `libbitgrain.so`/`.dylib` (cdylib) para bindings; `make lib-shared` para comprobar.
- **Bindings Python:** `bindings/python/bitgrain.py` (ctypes). Requiere `libbitgrain.so`/`.dylib` (ej. `make bitgrain`). Uso: cargar imagen con Pillow, pasar bytes a `encode_rgb`/`encode_rgba`/`decode`.
- **Bindings Go:** `bindings/go/` (cgo). Desde repo: `make bitgrain`; luego en `bindings/go`: `CGO_ENABLED=1 go build`. Ver `bindings/go/README.md`.

## Layout

```
bitgrain/
├── main.c           # Orquestación CLI
├── Makefile
├── includes/encoder.h
├── c/               # Modular: cli, path_utils, encode_cli, decode_cli, roundtrip_cli,
│                    # bg_utils, config, quant (SIMD), image_loader, image_writer, metrics, webp_io, platform
├── rust/            # encoder, decoder, dct, entropy, ffi; Cargo.toml listo crates.io
├── tests/           # integration.sh (CLI end-to-end)
└── bindings/
    ├── python/      # bitgrain.py (ctypes)
    └── go/          # bitgrain.go (cgo)
```

Tests: `./tests/integration.sh` (requiere build previo con libwebp).

## Formato .bg e interoperabilidad

Especificación pública en [FORMAT.md](FORMAT.md). Permite implementar lectores/escritores .bg en terceros.

## Benchmark

Comparar Bitgrain vs JPEG/WebP (tamaño y tiempo):

```bash
./scripts/benchmark.sh /ruta/imagen.jpg 85
```

Requiere: `cjpeg`/`djpeg` (libjpeg), `cwebp`/`dwebp` (libwebp).

## Roadmap

- **Formatos:** AVIF (libavif), TIFF (libtiff), RAW (opcional).
- **DCT/IDCT SIMD:** Implementado en `c/dct.c` (SSE2/NEON).
- **Streaming:** `decode_rle_one_block` permite decodificación bloque a bloque.
- **ICC/color management:** Extensión futura en FORMAT.md (v4+).
- **Progressive decode:** Reordenado de bitstream (roadmap).

---

**License:** GPL-3.0-or-later. See [LICENSE](LICENSE).
