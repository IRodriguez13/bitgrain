#!/bin/bash
# Benchmark: Bitgrain vs JPEG vs WebP
# Requires: bitgrain, cjpeg/djpeg (libjpeg), cwebp/dwebp (libwebp)
# Usage: ./scripts/benchmark.sh [image.jpg]

set -e
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BITGRAIN_DIR="$(dirname "$SCRIPT_DIR")"
IMAGE="${1:-}"
QUALITY="${2:-85}"

if [ -z "$IMAGE" ]; then
    echo "Usage: $0 <image.jpg|png> [quality 1-100]"
    echo "Example: $0 test_images/foto\\ 3.jpg 85"
    exit 1
fi

if [ ! -f "$IMAGE" ]; then
    echo "Error: image not found: $IMAGE"
    exit 1
fi

OUT="$BITGRAIN_DIR/bench_out"
mkdir -p "$OUT"
BASE="$(basename "$IMAGE" | sed 's/\.[^.]*$//')"

echo "=== Benchmark: $IMAGE (quality $QUALITY) ==="
echo ""

# Bitgrain round-trip
echo "[Bitgrain]"
BITGRAIN_BG="$OUT/${BASE}.bg"
BITGRAIN_OUT="$OUT/${BASE}_bitgrain.png"
START=$(date +%s%N)
"$BITGRAIN_DIR/bitgrain" -i "$IMAGE" -o "$BITGRAIN_BG" -q "$QUALITY" 2>/dev/null || true
"$BITGRAIN_DIR/bitgrain" -d -i "$BITGRAIN_BG" -o "$BITGRAIN_OUT" 2>/dev/null || true
END=$(date +%s%N)
BITGRAIN_MS=$(( (END - START) / 1000000 ))
BITGRAIN_BG_SIZE=$(stat -c%s "$BITGRAIN_BG" 2>/dev/null || echo 0)
BITGRAIN_ORIG=$(stat -c%s "$IMAGE" 2>/dev/null || echo 0)
echo "  .bg size: $BITGRAIN_BG_SIZE bytes"
echo "  Round-trip time: ${BITGRAIN_MS} ms"
echo ""

# JPEG (if cjpeg/djpeg available)
if command -v cjpeg >/dev/null 2>&1 && command -v djpeg >/dev/null 2>&1; then
    echo "[JPEG]"
    JPEG_OUT="$OUT/${BASE}.jpg"
    JPEG_RT="$OUT/${BASE}_jpeg_rt.png"
    START=$(date +%s%N)
    cjpeg -quality "$QUALITY" -outfile "$JPEG_OUT" "$IMAGE" 2>/dev/null || true
    djpeg -outfile "$JPEG_RT" "$JPEG_OUT" 2>/dev/null || true
    END=$(date +%s%N)
    JPEG_MS=$(( (END - START) / 1000000 ))
    JPEG_SIZE=$(stat -c%s "$JPEG_OUT" 2>/dev/null || echo 0)
    echo "  .jpg size: $JPEG_SIZE bytes"
    echo "  Encode+decode time: ${JPEG_MS} ms"
    echo ""
else
    echo "[JPEG] cjpeg/djpeg not found, skipping"
    echo ""
fi

# WebP (if cwebp/dwebp available)
if command -v cwebp >/dev/null 2>&1 && command -v dwebp >/dev/null 2>&1; then
    echo "[WebP]"
    WEBP_OUT="$OUT/${BASE}.webp"
    WEBP_RT="$OUT/${BASE}_webp_rt.png"
    START=$(date +%s%N)
    cwebp -q "$QUALITY" "$IMAGE" -o "$WEBP_OUT" 2>/dev/null || true
    dwebp "$WEBP_OUT" -o "$WEBP_RT" 2>/dev/null || true
    END=$(date +%s%N)
    WEBP_MS=$(( (END - START) / 1000000 ))
    WEBP_SIZE=$(stat -c%s "$WEBP_OUT" 2>/dev/null || echo 0)
    echo "  .webp size: $WEBP_SIZE bytes"
    echo "  Encode+decode time: ${WEBP_MS} ms"
    echo ""
else
    echo "[WebP] cwebp/dwebp not found, skipping"
    echo ""
fi

# Bitgrain metrics
if [ -f "$BITGRAIN_OUT" ]; then
    echo "[Bitgrain PSNR/SSIM]"
    "$BITGRAIN_DIR/bitgrain" -cd -i "$IMAGE" -o "$OUT/${BASE}_metrics.png" -y -m 2>/dev/null | grep -E "PSNR|SSIM" || true
fi

echo ""
echo "Output: $OUT"
