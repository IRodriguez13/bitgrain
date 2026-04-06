#!/bin/bash
# Benchmark: Bitgrain vs JPEG vs WebP (single image or directory)
# Requires: bitgrain, cjpeg/djpeg (libjpeg), cwebp/dwebp (libwebp)
# Usage: ./scripts/benchmark.sh <image|directory> [quality 1-100] [runs]

set -e
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BITGRAIN_DIR="$(dirname "$SCRIPT_DIR")"
IMAGE="${1:-}"
QUALITY="${2:-85}"
RUNS="${3:-3}"

if [ -z "$IMAGE" ]; then
    echo "Usage: $0 <image.jpg|png> [quality 1-100]"
    echo "Example: $0 test_images/foto\\ 3.jpg 85"
    exit 1
fi

if [ ! -e "$IMAGE" ]; then
    echo "Error: path not found: $IMAGE"
    exit 1
fi

OUT="$BITGRAIN_DIR/bench_out"
mkdir -p "$OUT"
BASE="$(basename "$IMAGE" | sed 's/\.[^.]*$//')"
TMP_WORK="$(mktemp -d /tmp/bitgrain-bench-XXXXXX)"

cleanup() {
    rm -rf "$TMP_WORK"
}
trap cleanup EXIT

if ! [ "$RUNS" -ge 1 ] 2>/dev/null; then
    echo "Error: runs must be >= 1"
    exit 1
fi

CSV="$OUT/benchmark.csv"
echo "image,codec,mode,runs,avg_ms,p50_ms,p95_ms,size_bytes" > "$CSV"

run_ms() {
    # run_ms <command...>
    local start end
    start=$(date +%s%N)
    "$@" >/dev/null 2>&1
    end=$(date +%s%N)
    echo $(( (end - start) / 1000000 ))
}

stats_ms() {
    # stats_ms "1 2 3"  -> "avg,p50,p95"
    python3 - "$1" <<'PY'
import sys
import math
vals=[int(x) for x in sys.argv[1].split() if x.strip()]
if not vals:
    print("0,0,0")
    raise SystemExit
s=sorted(vals)
n=len(s)
avg=round(sum(s)/n,2)
if n % 2 == 1:
    med=float(s[n//2])
else:
    med=(s[n//2 - 1] + s[n//2]) / 2.0
idx=max(0, math.ceil(0.95*n)-1)
p95=float(s[idx])
print(f"{avg:.2f},{med:.2f},{p95:.2f}")
PY
}

bench_one() {
    local img="$1"
    local base
    base="$(basename "$img" | sed 's/\.[^.]*$//')"
    if [ ! -s "$img" ]; then
        echo "⚠️ Skipping empty file: $img"
        return
    fi

    echo "=== Benchmark: $img (quality $QUALITY, runs $RUNS) ==="
    echo ""

    # Bitgrain: encode-only and roundtrip.
    echo "[Bitgrain]"
    local safe
    safe="$(printf "%s" "$img" | tr '/ ' '__')"
    local bg_file="$TMP_WORK/${safe}.bg"
    local bg_out="$TMP_WORK/${safe}_bitgrain.bmp"
    local bg_enc_times=""
    local bg_dec_times=""
    local bg_rt_times=""
    for _ in $(seq 1 "$RUNS"); do
        local t_enc t_dec
        t_enc=$(run_ms "$BITGRAIN_DIR/bitgrain" encode "$img" -o "$bg_file" -q "$QUALITY" -y)
        t_dec=$(run_ms "$BITGRAIN_DIR/bitgrain" decode "$bg_file" -o "$bg_out" -y)
        bg_enc_times="$bg_enc_times $t_enc"
        bg_dec_times="$bg_dec_times $t_dec"
        bg_rt_times="$bg_rt_times $((t_enc + t_dec))"
    done
    local bg_size
    bg_size=$(stat -c%s "$bg_file" 2>/dev/null || echo 0)
    local bg_enc_avg bg_enc_p50 bg_enc_p95
    local bg_dec_avg bg_dec_p50 bg_dec_p95
    local bg_rt_avg bg_rt_p50 bg_rt_p95
    IFS=',' read -r bg_enc_avg bg_enc_p50 bg_enc_p95 <<<"$(stats_ms "$bg_enc_times")"
    IFS=',' read -r bg_dec_avg bg_dec_p50 bg_dec_p95 <<<"$(stats_ms "$bg_dec_times")"
    IFS=',' read -r bg_rt_avg bg_rt_p50 bg_rt_p95 <<<"$(stats_ms "$bg_rt_times")"
    echo "  .bg size: $bg_size bytes"
    echo "  Encode avg/p50/p95: $bg_enc_avg / $bg_enc_p50 / $bg_enc_p95 ms"
    echo "  Decode avg/p50/p95: $bg_dec_avg / $bg_dec_p50 / $bg_dec_p95 ms"
    echo "  Round-trip avg/p50/p95: $bg_rt_avg / $bg_rt_p50 / $bg_rt_p95 ms"
    echo "$img,bitgrain,encode,$RUNS,$bg_enc_avg,$bg_enc_p50,$bg_enc_p95,$bg_size" >> "$CSV"
    echo "$img,bitgrain,decode,$RUNS,$bg_dec_avg,$bg_dec_p50,$bg_dec_p95,$bg_size" >> "$CSV"
    echo "$img,bitgrain,roundtrip,$RUNS,$bg_rt_avg,$bg_rt_p50,$bg_rt_p95,$bg_size" >> "$CSV"
    echo ""

    # JPEG: encode-only and roundtrip.
    if command -v cjpeg >/dev/null 2>&1 && command -v djpeg >/dev/null 2>&1; then
        echo "[JPEG]"
        local jpg_out="$TMP_WORK/${safe}.jpg"
        local jpg_rt="$TMP_WORK/${safe}_jpeg_rt.bmp"
        local jpg_src_ppm="$TMP_WORK/${safe}_jpeg_src.ppm"
        case "${img##*.}" in
            jpg|jpeg|JPG|JPEG)
                djpeg -outfile "$jpg_src_ppm" "$img" >/dev/null 2>&1 || true
                ;;
            *)
                convert "$img" "$jpg_src_ppm" >/dev/null 2>&1 || true
                ;;
        esac
        local jpg_enc_times=""
        local jpg_dec_times=""
        local jpg_rt_times=""
        if [ -s "$jpg_src_ppm" ]; then
            for _ in $(seq 1 "$RUNS"); do
                local t_enc t_dec
                t_enc=$(run_ms cjpeg -quality "$QUALITY" -outfile "$jpg_out" "$jpg_src_ppm")
                t_dec=$(run_ms djpeg -bmp -outfile "$jpg_rt" "$jpg_out")
                jpg_enc_times="$jpg_enc_times $t_enc"
                jpg_dec_times="$jpg_dec_times $t_dec"
                jpg_rt_times="$jpg_rt_times $((t_enc + t_dec))"
            done
        fi
        local jpg_size
        jpg_size=$(stat -c%s "$jpg_out" 2>/dev/null || echo 0)
        if [ "$jpg_size" -le 0 ]; then
            echo "  ⚠️ JPEG skipped: failed to build PPM source for cjpeg."
        else
            local jpg_enc_avg jpg_enc_p50 jpg_enc_p95
            local jpg_dec_avg jpg_dec_p50 jpg_dec_p95
            local jpg_rt_avg jpg_rt_p50 jpg_rt_p95
            IFS=',' read -r jpg_enc_avg jpg_enc_p50 jpg_enc_p95 <<<"$(stats_ms "$jpg_enc_times")"
            IFS=',' read -r jpg_dec_avg jpg_dec_p50 jpg_dec_p95 <<<"$(stats_ms "$jpg_dec_times")"
            IFS=',' read -r jpg_rt_avg jpg_rt_p50 jpg_rt_p95 <<<"$(stats_ms "$jpg_rt_times")"
            echo "  .jpg size: $jpg_size bytes"
            echo "  Encode avg/p50/p95: $jpg_enc_avg / $jpg_enc_p50 / $jpg_enc_p95 ms"
            echo "  Decode avg/p50/p95: $jpg_dec_avg / $jpg_dec_p50 / $jpg_dec_p95 ms"
            echo "  Encode+decode avg/p50/p95: $jpg_rt_avg / $jpg_rt_p50 / $jpg_rt_p95 ms"
            echo "$img,jpeg,encode,$RUNS,$jpg_enc_avg,$jpg_enc_p50,$jpg_enc_p95,$jpg_size" >> "$CSV"
            echo "$img,jpeg,decode,$RUNS,$jpg_dec_avg,$jpg_dec_p50,$jpg_dec_p95,$jpg_size" >> "$CSV"
            echo "$img,jpeg,roundtrip,$RUNS,$jpg_rt_avg,$jpg_rt_p50,$jpg_rt_p95,$jpg_size" >> "$CSV"
        fi
        echo ""
    else
        echo "[JPEG] cjpeg/djpeg not found, skipping"
        echo ""
    fi

    # WebP: encode-only and roundtrip.
    if command -v cwebp >/dev/null 2>&1 && command -v dwebp >/dev/null 2>&1; then
        echo "[WebP]"
        local webp_out="$TMP_WORK/${safe}.webp"
        local webp_rt="$TMP_WORK/${safe}_webp_rt.bmp"
        local webp_enc_times=""
        local webp_dec_times=""
        local webp_rt_times=""
        for _ in $(seq 1 "$RUNS"); do
            local t_enc t_dec
            t_enc=$(run_ms cwebp -quiet -q "$QUALITY" "$img" -o "$webp_out")
            t_dec=$(run_ms dwebp -bmp "$webp_out" -o "$webp_rt")
            webp_enc_times="$webp_enc_times $t_enc"
            webp_dec_times="$webp_dec_times $t_dec"
            webp_rt_times="$webp_rt_times $((t_enc + t_dec))"
        done
        local webp_size
        webp_size=$(stat -c%s "$webp_out" 2>/dev/null || echo 0)
        local webp_enc_avg webp_enc_p50 webp_enc_p95
        local webp_dec_avg webp_dec_p50 webp_dec_p95
        local webp_rt_avg webp_rt_p50 webp_rt_p95
        IFS=',' read -r webp_enc_avg webp_enc_p50 webp_enc_p95 <<<"$(stats_ms "$webp_enc_times")"
        IFS=',' read -r webp_dec_avg webp_dec_p50 webp_dec_p95 <<<"$(stats_ms "$webp_dec_times")"
        IFS=',' read -r webp_rt_avg webp_rt_p50 webp_rt_p95 <<<"$(stats_ms "$webp_rt_times")"
        echo "  .webp size: $webp_size bytes"
        echo "  Encode avg/p50/p95: $webp_enc_avg / $webp_enc_p50 / $webp_enc_p95 ms"
        echo "  Decode avg/p50/p95: $webp_dec_avg / $webp_dec_p50 / $webp_dec_p95 ms"
        echo "  Encode+decode avg/p50/p95: $webp_rt_avg / $webp_rt_p50 / $webp_rt_p95 ms"
        echo "$img,webp,encode,$RUNS,$webp_enc_avg,$webp_enc_p50,$webp_enc_p95,$webp_size" >> "$CSV"
        echo "$img,webp,decode,$RUNS,$webp_dec_avg,$webp_dec_p50,$webp_dec_p95,$webp_size" >> "$CSV"
        echo "$img,webp,roundtrip,$RUNS,$webp_rt_avg,$webp_rt_p50,$webp_rt_p95,$webp_size" >> "$CSV"
        echo ""
    else
        echo "[WebP] cwebp/dwebp not found, skipping"
        echo ""
    fi
}

if [ -d "$IMAGE" ]; then
    for img in "$IMAGE"/*; do
        [ -f "$img" ] || continue
        case "${img##*.}" in
            jpg|jpeg|png|bmp|tga|webp|JPG|JPEG|PNG|BMP|TGA|WEBP)
                bench_one "$img"
                ;;
            *)
                ;;
        esac
    done
else
    bench_one "$IMAGE"
fi

python3 - "$CSV" <<'PY'
import csv
import sys
from collections import defaultdict

csv_path = sys.argv[1]
rows = list(csv.DictReader(open(csv_path, newline="", encoding="utf-8")))
if not rows:
    raise SystemExit

by = defaultdict(dict)
images = set()
for r in rows:
    image = r["image"]
    mode = r["mode"]
    codec = r["codec"]
    images.add(image)
    by[(image, mode)][codec] = r

def cell(r):
    return f'{r["avg_ms"]}ms (p50 {r["p50_ms"]}, p95 {r["p95_ms"]}, {r["size_bytes"]}B)'

for mode in ("encode", "decode", "roundtrip"):
    print(f"\n=== Summary ({mode}) ===")
    print("image | bitgrain | jpeg | webp")
    print("-" * 120)
    for image in sorted(images):
        d = by.get((image, mode), {})
        bg = cell(d["bitgrain"]) if "bitgrain" in d else "-"
        jp = cell(d["jpeg"]) if "jpeg" in d else "-"
        wp = cell(d["webp"]) if "webp" in d else "-"
        print(f"{image} | {bg} | {jp} | {wp}")
PY

echo "Output: $OUT"
echo "CSV: $CSV"
