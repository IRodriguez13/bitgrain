#!/bin/bash
# Integration tests for bitgrain CLI. Run from repo root.
set -e
cd "$(dirname "$0")/.."
BIN="./bitgrain"

# Build if needed
if [ ! -x "$BIN" ]; then
    make bitgrain 2>/dev/null || { echo "Build failed (need libwebp)"; exit 1; }
fi

# Create minimal 8x8 grayscale PGM
mkdir -p tests/out
printf 'P5\n8 8\n255\n' > tests/out/mini.pgm
for i in {1..64}; do printf '\x80'; done >> tests/out/mini.pgm

echo "=== Encode ==="
$BIN -i tests/out/mini.pgm -o tests/out/mini.bg -y
test -f tests/out/mini.bg || { echo "Encode failed"; exit 1; }

echo "=== Decode ==="
$BIN -d -i tests/out/mini.bg -o tests/out/mini_decoded.pgm -y
test -f tests/out/mini_decoded.pgm || { echo "Decode failed"; exit 1; }

echo "=== Round-trip ==="
$BIN -cd -i tests/out/mini.pgm -o tests/out/mini_rt.pgm -y -m
test -f tests/out/mini_rt.pgm || { echo "Round-trip failed"; exit 1; }

echo "=== All integration tests passed ==="
