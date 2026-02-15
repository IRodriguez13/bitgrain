# Bitgrain (.bg) Format Specification

This document specifies the .bg binary format for interoperability. Third-party implementations can read and write .bg files using this specification.

## Overview

Bitgrain is a block-based image codec using 8×8 DCT, quantization, and RLE entropy coding. It supports grayscale (1 channel), RGB (3 channels), and RGBA (4 channels).

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
| 2      | 1    | version  | 1=grayscale, 2=RGB, 3=RGBA |
| 3      | 4    | width    | Image width (uint32 LE) |
| 7      | 4    | height   | Image height (uint32 LE) |
| 11     | 1    | quality  | Quantization quality 1–100; 0 means default 50 |

**Constraints:** width and height must be in [1, 65536]. Version must be 1, 2, or 3.

## Block Layout

The image is divided into 8×8 blocks in scan order (left-to-right, top-to-bottom). Blocks in the last row/column may be partially filled; edge pixels are replicated.

- **Grayscale (version 1):** One plane of blocks (Y).
- **RGB (version 2):** Three planes: Y (luma), Cb, Cr. Each plane has `ceil(width/8) * ceil(height/8)` blocks.
- **RGBA (version 3):** Four planes: R, G, B, A. Same block count per plane.

## Block Encoding (RLE)

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

## Quantization

Quality maps to a scaled JPEG luminance table. Formula for table entry `i`:

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

- **Version 4+:** Extended header with optional chunks.
- **Progressive decoding:** Reordered bitstream for coarse-to-fine decode (roadmap).

## Reference Implementation

- Encoder/decoder: this repository (Rust core, C FFI).
- C API: `includes/encoder.h` — `bitgrain_encode_*`, `bitgrain_decode`.
