#!/bin/bash
# Compare Bitgrain vs JPEG one-by-one (size + speed).
# Usage:
#   ./scripts/compare_bg_vs_jpeg.sh <image|directory> [quality 1-100] [runs>=1]

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
INPUT_PATH="${1:-}"
QUALITY="${2:-85}"
RUNS="${3:-3}"
OUT_DIR="$ROOT_DIR/bench_out/one_by_one"
CSV="$OUT_DIR/bg_vs_jpeg.csv"
TMP_WORK="$(mktemp -d /tmp/bitgrain-compare-XXXXXX)"

cleanup() {
    rm -rf "$TMP_WORK"
}
trap cleanup EXIT

if [ -z "$INPUT_PATH" ]; then
    echo "Usage: $0 <image|directory> [quality 1-100] [runs>=1]"
    exit 1
fi

if [ ! -e "$INPUT_PATH" ]; then
    echo "Error: path not found: $INPUT_PATH"
    exit 1
fi

if ! [ "$RUNS" -ge 1 ] 2>/dev/null; then
    echo "Error: runs must be >= 1"
    exit 1
fi

if ! command -v cjpeg >/dev/null 2>&1 || ! command -v djpeg >/dev/null 2>&1; then
    echo "Error: cjpeg/djpeg are required"
    exit 1
fi

if ! command -v convert >/dev/null 2>&1; then
    echo "Error: ImageMagick 'convert' is required"
    exit 1
fi

mkdir -p "$OUT_DIR"
echo "image,input_bytes,bg_bytes,jpeg_bytes,bg_to_jpeg_ratio,bg_minus_jpeg_bytes,bg_minus_jpeg_pct,bg_enc_avg_ms,bg_enc_p50_ms,bg_enc_p95_ms,bg_dec_avg_ms,bg_dec_p50_ms,bg_dec_p95_ms,jpeg_enc_avg_ms,jpeg_enc_p50_ms,jpeg_enc_p95_ms,jpeg_dec_avg_ms,jpeg_dec_p50_ms,jpeg_dec_p95_ms" > "$CSV"

run_ms() {
    local start end
    start=$(date +%s%N)
    "$@" >/dev/null 2>&1
    end=$(date +%s%N)
    echo $(( (end - start) / 1000000 ))
}

stats_ms() {
    python3 - "$1" <<'PY'
import sys, math
vals=[int(x) for x in sys.argv[1].split() if x.strip()]
if not vals:
    print("0,0,0")
    raise SystemExit
s=sorted(vals)
n=len(s)
avg=sum(s)/n
if n % 2:
    p50=float(s[n//2])
else:
    p50=(s[n//2 - 1] + s[n//2]) / 2.0
idx=max(0, math.ceil(0.95*n)-1)
p95=float(s[idx])
print(f"{avg:.2f},{p50:.2f},{p95:.2f}")
PY
}

is_supported() {
    case "${1##*.}" in
        jpg|jpeg|png|bmp|tga|webp|JPG|JPEG|PNG|BMP|TGA|WEBP) return 0 ;;
        *) return 1 ;;
    esac
}

compare_one() {
    local img="$1"
    local base safe
    base="$(basename "$img" | sed 's/\.[^.]*$//')"
    safe="$(printf "%s" "$img" | tr '/ ' '__')"

    if [ ! -s "$img" ]; then
        echo "Skipping empty file: $img"
        return
    fi

    local bg_file="$TMP_WORK/${safe}.bg"
    local bg_dec_bmp="$TMP_WORK/${safe}_bg.bmp"
    local jpg_src_ppm="$TMP_WORK/${safe}_src.ppm"
    local jpg_file="$TMP_WORK/${safe}.jpg"
    local jpg_dec_bmp="$TMP_WORK/${safe}_jpg.bmp"

    # Build PPM source for cjpeg from any input format.
    case "${img##*.}" in
        jpg|jpeg|JPG|JPEG)
            djpeg -outfile "$jpg_src_ppm" "$img" >/dev/null 2>&1 || true
            ;;
        *)
            convert "$img" "$jpg_src_ppm" >/dev/null 2>&1 || true
            ;;
    esac
    if [ ! -s "$jpg_src_ppm" ]; then
        echo "Skipping (failed to build PPM): $img"
        return
    fi

    local bg_enc_times="" bg_dec_times="" jpg_enc_times="" jpg_dec_times=""
    for _ in $(seq 1 "$RUNS"); do
        local t1 t2 t3 t4
        t1=$(run_ms "$ROOT_DIR/bitgrain" encode "$img" -o "$bg_file" -q "$QUALITY" -y)
        t2=$(run_ms "$ROOT_DIR/bitgrain" decode "$bg_file" -o "$bg_dec_bmp" -y)
        t3=$(run_ms cjpeg -quality "$QUALITY" -outfile "$jpg_file" "$jpg_src_ppm")
        t4=$(run_ms djpeg -bmp -outfile "$jpg_dec_bmp" "$jpg_file")
        bg_enc_times="$bg_enc_times $t1"
        bg_dec_times="$bg_dec_times $t2"
        jpg_enc_times="$jpg_enc_times $t3"
        jpg_dec_times="$jpg_dec_times $t4"
    done

    local input_bytes bg_bytes jpeg_bytes
    input_bytes=$(stat -c%s "$img")
    bg_bytes=$(stat -c%s "$bg_file")
    jpeg_bytes=$(stat -c%s "$jpg_file")

    local bg_enc_avg bg_enc_p50 bg_enc_p95
    local bg_dec_avg bg_dec_p50 bg_dec_p95
    local jp_enc_avg jp_enc_p50 jp_enc_p95
    local jp_dec_avg jp_dec_p50 jp_dec_p95
    IFS=',' read -r bg_enc_avg bg_enc_p50 bg_enc_p95 <<<"$(stats_ms "$bg_enc_times")"
    IFS=',' read -r bg_dec_avg bg_dec_p50 bg_dec_p95 <<<"$(stats_ms "$bg_dec_times")"
    IFS=',' read -r jp_enc_avg jp_enc_p50 jp_enc_p95 <<<"$(stats_ms "$jpg_enc_times")"
    IFS=',' read -r jp_dec_avg jp_dec_p50 jp_dec_p95 <<<"$(stats_ms "$jpg_dec_times")"

    local ratio delta_bytes delta_pct
    ratio=$(python3 - "$bg_bytes" "$jpeg_bytes" <<'PY'
import sys
bg=int(sys.argv[1]); jp=int(sys.argv[2])
print(f"{(bg/jp):.4f}" if jp > 0 else "0.0000")
PY
)
    delta_bytes=$((bg_bytes - jpeg_bytes))
    delta_pct=$(python3 - "$bg_bytes" "$jpeg_bytes" <<'PY'
import sys
bg=int(sys.argv[1]); jp=int(sys.argv[2])
print(f"{((bg-jp)*100.0/jp):.2f}" if jp > 0 else "0.00")
PY
)

    echo "$img,$input_bytes,$bg_bytes,$jpeg_bytes,$ratio,$delta_bytes,$delta_pct,$bg_enc_avg,$bg_enc_p50,$bg_enc_p95,$bg_dec_avg,$bg_dec_p50,$bg_dec_p95,$jp_enc_avg,$jp_enc_p50,$jp_enc_p95,$jp_dec_avg,$jp_dec_p50,$jp_dec_p95" >> "$CSV"
}

if [ -d "$INPUT_PATH" ]; then
    for img in "$INPUT_PATH"/*; do
        [ -f "$img" ] || continue
        if is_supported "$img"; then
            compare_one "$img"
        fi
    done
else
    if is_supported "$INPUT_PATH"; then
        compare_one "$INPUT_PATH"
    else
        echo "Error: unsupported extension: $INPUT_PATH"
        exit 1
    fi
fi

python3 - "$CSV" <<'PY'
import csv, sys
rows=list(csv.DictReader(open(sys.argv[1], newline="", encoding="utf-8")))
if not rows:
    print("No rows generated.")
    raise SystemExit

print("\n=== Bitgrain vs JPEG (one-by-one) ===")
print("image | sizes (bg/jpg, ratio, delta%) | encode avg (bg/jpg) | decode avg (bg/jpg)")
print("-" * 130)
for r in rows:
    print(
        f'{r["image"]} | '
        f'{r["bg_bytes"]}/{r["jpeg_bytes"]} B ({r["bg_to_jpeg_ratio"]}x, {r["bg_minus_jpeg_pct"]}%) | '
        f'{r["bg_enc_avg_ms"]}/{r["jpeg_enc_avg_ms"]} ms | '
        f'{r["bg_dec_avg_ms"]}/{r["jpeg_dec_avg_ms"]} ms'
    )

avg_ratio=sum(float(r["bg_to_jpeg_ratio"]) for r in rows)/len(rows)
avg_delta=sum(float(r["bg_minus_jpeg_pct"]) for r in rows)/len(rows)
print("\n--- Aggregate ---")
print(f"images: {len(rows)}")
print(f"avg bg/jpg size ratio: {avg_ratio:.4f}x")
print(f"avg size delta pct (bg vs jpg): {avg_delta:.2f}%")
print(f"csv: {sys.argv[1]}")
PY
