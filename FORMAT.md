# Bitgrain (.bg) Format Specification

This document specifies the .bg binary format for interoperability. Third-party implementations can read and write .bg files using this specification.

## Overview

Bitgrain is a block-based image codec using 8×8 DCT, quantization, and entropy coding. It supports grayscale (1 channel), RGB (3 channels), and RGBA (4 channels).

## File Structure

```
+----------+----------+----------+------------------+
|  Header  |  Plane 0 |  Plane 1 |  ... (if RGB/RGBA) |
+----------+----------+----------+------------------+
```

## Header (12 bytes)

| Offset | Size | Field    | Description |
|--------|------|----------|-------------|
| 0      | 2    | magic    | `0x42 0x47` ("BG") |
| 2      | 1    | version  | See version table below |
| 3      | 4    | width    | Image width (uint32 LE) |
| 7      | 4    | height   | Image height (uint32 LE) |
| 11     | 1    | quality  | Quantization quality 1–100; 0 means default 50 |

## Block Layout

The image is divided into 8×8 blocks in scan order (left-to-right, top-to-bottom). Blocks in the last row/column may be partially filled; edge pixels are replicated.

- **v1 (grayscale RLE):** One full-resolution plane.
- **v2/v3 (legacy RLE):** 3 or 4 full-resolution planes.
- **v4+ (YCbCr):**
  - RGB profiles: Y full-resolution + Cb/Cr at 4:2:0 (`ceil(w/2) x ceil(h/2)`).
  - RGBA profiles: same Y/Cb/Cr plus full-resolution alpha plane.

## Block Encoding

### RLE path (v1/v2/v3)

Each block is encoded as:

1. **DC coefficient:** 2 bytes, little-endian int16.
2. **AC coefficients:** Repeated (run, level) pairs until EOB.
   - `run`: 1 byte, number of zeros before this coefficient (0–255).
   - `level`: 2 bytes, little-endian int16, the coefficient value.
3. **End-of-block:** `run=0xFF`, `level=0` (3 bytes: `0xFF 0x00 0x00`).

Coefficients are in **zigzag order** (JPEG order):

```
 0  1  8 16  9  2  3 10
17 24 32 25 18 11  4  5
12 19 26 33 40 48 41 34
...
```

### Huffman path (v4+)

For each plane:

1. 4-byte little-endian plane payload length.
2. Bitstream payload (MSB-first).
3. Byte-stuffing `0xFF -> 0xFF 0x00` inside entropy payload.
4. Final plane flush pads with 1s (JPEG convention).

The decoder uses the per-plane length to jump exactly to the next plane boundary.

## Quantization

Quality maps to scaled JPEG-like quantization tables (luma and chroma). Newer versions apply increasingly perceptual weighting profiles to close file-size gap versus JPEG.

Base scaling formula:

```
scaled[i] = clamp((base[i] * quality / 100 + 50) / 100, 1, 255)
```

Default table (quality 50):

```
16, 11, 10, 16, 24, 40, 51, 61,
12, 12, 14, 19, 26, 58, 60, 55,
14, 13, 16, 24, 40, 57, 69, 56,
14, 17, 22, 29, 51, 87, 80, 62,
18, 22, 37, 56, 68,109,103, 77,
24, 35, 55, 64, 81,104,113, 92,
49, 64, 78, 87,103,121,120,101,
72, 92, 95, 98,112,100,103, 99
```

Dequantization: `coef[i] *= table[i]` (before IDCT).

## DCT/IDCT

Standard 8×8 DCT-II (forward) and IDCT-II (inverse). Input pixels are centered (0–255 → -128..127). Output coefficients are rounded to int16.

## Optional ICC Trailer

After all plane blocks, an optional trailer may appear:

| Bytes | Field   | Description |
|-------|---------|-------------|
| 3     | magic   | `0x42 0x47 0x78` ("BGx") |
| 1     | type    | Chunk type: 1 = ICC profile |
| 4     | length  | ICC data length (uint32 LE) |
| N     | data    | ICC profile bytes |

Decoders that don't support ICC skip the trailer. Encoders write it only when an ICC profile is provided.

## Extensions (Future)

- Progressive decode / multi-pass refinement (roadmap).

## Reference Implementation

- Encoder/decoder: this repository (Rust core, C FFI).
- C API: `includes/encoder.h` — `bitgrain_encode_*`, `bitgrain_decode`.
