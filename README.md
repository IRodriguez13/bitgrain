# Bitgrain

**v1.0.0** — Image compressor (JPEG-like). Encodes images to a custom `.bg` stream; decodes back to pixels or standard image files. Grayscale and RGB. Single CLI and C API for encode/decode.

## Build

- **Requirements:** Rust (stable), GCC (C11), make.

```bash
make build
```

Produces the `bitgrain` binary. `make clean` then `make build` for a full rebuild.

---

## CLI

| Option | Description |
|--------|-------------|
| `-i <file>` | Input (image or .bg depending on mode) |
| `-o <file>` | Output (format from extension: .jpg, .png, .pgm) |
| `-d` | Decode: .bg → image |
| `-cd` | Round-trip: encode + decode in memory, write image (no .bg file) |
| `-q <1-100>` | Encode quality (default 85) |
| `-Q <1-100>` | Output image quality for decode/round-trip (default 85, for JPG) |
| `-y` | Overwrite output without prompting |
| `-v` | Print version and exit |
| `-h` | Help |

**Encode (image → .bg):**

```bash
bitgrain -i foto.jpg -o out.bg
bitgrain foto.png              # writes foto.bg
```

**Decode (.bg → image):**

```bash
bitgrain -d -i out.bg -o out.jpg
bitgrain -d out.bg             # writes out.jpg by default
```

**Round-trip (image → encode → decode → image):**

```bash
bitgrain -cd -i foto.jpg -o reconstruida.jpg
```

Input: JPEG, PNG, BMP, PGM, TGA, etc. (via [stb_image](https://github.com/nothings/stb)). Output format is determined by the `-o` extension (.jpg, .png, .pgm).

---

## Format .bg (v1.0)

- **Header (12 bytes):** magic `"BG"` (2), version (1: grayscale, 2: RGB), width (4, LE), height (4, LE), quality (1, 1–100; 0 means 50).
- **Payload:** blocks in scan order. Per block: DC (2 bytes int16 LE), then AC in zigzag as (run: 1 byte, level: 2 bytes int16 LE). EOB: run=0xFF, level=0.

Dimensions need not be multiples of 8; edge blocks are truncated.

---

## Pipeline (summary)

**Encode:** Load image → 8×8 blocks (centered: −128) → DCT → quantize (table scaled by quality) → RLE (DC + zigzag AC) → write header + stream.

**Decode:** Read header → RLE → dequantize → IDCT → +128, clamp → reassemble image.

DCT/IDCT use a reference implementation. Parallel blocks (Rayon) for encode and decode.

---

## C API

Declared in `includes/encoder.h`. Link with `rust/target/release/libbitgrain.a` and C objects; add `-lpthread -ldl -lm`.

**Encode (grayscale):** `bitgrain_encode_grayscale(pixels, width, height, out_buf, out_cap, &out_len, quality)`  
**Decode (grayscale):** `bitgrain_decode_grayscale(bg_buf, bg_size, pixels, pixels_cap, &width, &height)`

RGB variants and helpers for loading/writing images are in the same header. Intended use: backend for capture pipelines (compress to .bg for storage/transfer; decode to pixels or export to JPEG/PNG for display).

---

## Layout

```
bitgrain/
├── main.c           # CLI
├── Makefile
├── includes/encoder.h
├── c/               # quant, image_loader, image_writer (stb)
└── rust/src/        # lib, ffi, encoder, decoder, block, blockizer, dct, entropy, bitstream, zigzag
```

---

**License:** GPL-3.0-or-later. See [LICENSE](LICENSE).
