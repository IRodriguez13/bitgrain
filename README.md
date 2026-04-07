# Bitgrain

Image compressor (JPEG-like). Encodes to a custom `.bg` stream; decodes to pixels or standard image files. Grayscale, RGB, RGBA. CLI + C API (FFI-backed) with deterministic mode support.

Current focus in `2.0.0`: close the size gap vs JPEG while preserving practical encode/decode speed.

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
- `v6-v7`: chroma AC JPEG table for Cb/Cr
- `v8-v9`: perceptual quantization profile
- `v10-v11`: perceptual + chroma AC + DC delta (JPEG-like DC prediction)
- `v12-v13`: stronger perceptual profile
- `v14-v15`: aggressive perceptual profile
- `v16-v17`: very aggressive perceptual profile
- `v18-v19`: ultra perceptual + AC sparsify profile (best compression in current branch)

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

Comparar Bitgrain vs JPEG/WebP (tamaño y tiempo, `avg/p50/p95`):

```bash
./scripts/benchmark.sh /ruta/imagen.jpg 85
```

Requiere: `cjpeg`/`djpeg` (libjpeg), `cwebp`/`dwebp` (libwebp).

Comparación directa Bitgrain vs JPEG, imagen por imagen:

```bash
./scripts/compare_bg_vs_jpeg.sh /ruta/imagenes 85 3
```

CSV outputs:

- `bench_out/benchmark.csv`
- `bench_out/one_by_one/bg_vs_jpeg.csv`

Temporary intermediate files are created under `/tmp` and removed automatically.

### Snapshot (latest local run, quality 85)

Setup used for the numbers below:

- Bitgrain profile: `v18/v19` (ultra perceptual + AC sparsify)
- Runs: `5`
- Codecs: Bitgrain vs JPEG (`cjpeg/djpeg`) vs JPEG2000 (`opj_compress/opj_decompress -r 20`)
- Images: `weic2517d.jpg` and `potm2411b.jpg`

| Image | Codec | Size (bytes) | Encode avg ms | Decode avg ms |
|------|------|-------------:|--------------:|--------------:|
| weic2517d.jpg | Bitgrain | 638,085 | 131.06 | 179.92 |
| weic2517d.jpg | JPEG (cjpeg) | 1,269,030 | 30.38 | 141.97 |
| weic2517d.jpg | JPEG2000 (opj) | 1,530,779 | 1817.07 | 460.80 |
| potm2411b.jpg | Bitgrain | 60,011 | 16.43 | 17.11 |
| potm2411b.jpg | JPEG (cjpeg) | 173,980 | 3.68 | 13.63 |
| potm2411b.jpg | JPEG2000 (opj) | 117,857 | 227.97 | 38.66 |

Quick ranking from this snapshot:

- **Compression (smaller is better):** Bitgrain > JPEG > JPEG2000
- **Encode speed (faster is better):** JPEG > Bitgrain >>> JPEG2000
- **Decode speed (faster is better):** JPEG > Bitgrain >> JPEG2000

Why this still sells:

- Bitgrain is significantly smaller than JPEG in the measured sample while remaining in practical encode/decode times.
- Against JPEG2000, Bitgrain is both much faster and smaller in this setup.

Use these numbers as a reproducible project snapshot, not an absolute universal claim. Always validate with your own dataset and quality targets.

## Roadmap

- **Formatos:** AVIF (libavif), TIFF (libtiff), RAW (opcional).
- **DCT/IDCT SIMD:** Implementado en `c/dct.c` (SSE2/NEON).
- **Streaming:** `decode_rle_one_block` permite decodificación bloque a bloque.
- **ICC/color management:** Extensión futura en FORMAT.md (v4+).
- **Progressive decode:** Reordenado de bitstream (roadmap).

---

**License:** GPL-3.0-or-later. See [LICENSE](LICENSE).
