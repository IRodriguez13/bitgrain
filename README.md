# Bitgrain

Image compressor (JPEG-like). Encodes to a custom `.bg` stream; decodes to pixels or standard image files. Grayscale, RGB, RGBA. CLI and C API.

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

| Option | Description |
|--------|-------------|
| `-i <path>` | Input file or directory |
| `-o <path>` | Output file or directory |
| `-d` | Decode: .bg → image |
| `-cd` | Round-trip: encode + decode in memory, write image |
| `-q <1-100>` | Encode quality (default 85) |
| `-Q <1-100>` | Output JPG/WebP quality (default 85) |
| `-m` | Round-trip: print PSNR and SSIM |
| `-y` | Overwrite |
| `-v`, `-h` | Version, help |

Input formats: JPEG, PNG, BMP, GIF, TGA, PGM, PSD, HDR, WebP (stb_image + libwebp). Output: extension of `-o` (.jpg, .png, .pgm, .webp). **Transparencia/canales alfa:** RGBA soportado (PNG/WebP con alpha → .bg versión 3; decode → 4 canales).

## Format .bg

Header 12 bytes: magic "BG", version (1=gray, 2=RGB, 3=RGBA), width (4 LE), height (4 LE), quality (1). Payload: blocks in scan order; per block DC (2 bytes), AC zigzag (run 1 byte, level 2 bytes), EOB run=0xFF level=0.

## C API

`includes/encoder.h`. Encode: `bitgrain_encode_grayscale`, `bitgrain_encode_rgb`, `bitgrain_encode_rgba`. Decode: `bitgrain_decode(buf, size, pixels, cap, &w, &h, &channels)`. Channels 1, 3, or 4. Link `libbitgrain.a` and C objects; add `-lpthread -ldl -lm -lwebp`. Load/save helpers in `c/image_loader.h`, `c/image_writer.h`.

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
