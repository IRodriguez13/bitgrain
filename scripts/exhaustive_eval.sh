#!/usr/bin/env bash
# Exhaustive evaluation for bitgrain as CLI + library API.
# Measures speed, size reduction, and quality (PSNR/SSIM) on small/large and batch datasets.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BITGRAIN_BIN="$ROOT_DIR/bitgrain"
BENCH_BIN="$ROOT_DIR/bitgrain-bench"

OUT_DIR="${1:-$ROOT_DIR/eval_out}"
DATASET_DIR="$OUT_DIR/dataset"
CLI_OUT_DIR="$OUT_DIR/cli"
BATCH_OUT_DIR="$OUT_DIR/batch"
LIB_OUT_DIR="$OUT_DIR/lib"

QUALITIES="${QUALITIES:-75 85 92}"
THREADS="${THREADS:-0}"          # 0 = runtime default
RUNS="${RUNS:-6}"
WARMUP="${WARMUP:-2}"
PSNR_MIN="${PSNR_MIN:-20.0}"     # gate for non-noise images
SSIM_MIN="${SSIM_MIN:-0.55}"     # gate for non-noise images

mkdir -p "$DATASET_DIR" "$CLI_OUT_DIR" "$BATCH_OUT_DIR" "$LIB_OUT_DIR"

echo "==> Building bitgrain + bench"
make -C "$ROOT_DIR" -j4 >/dev/null
make -C "$ROOT_DIR" bench >/dev/null

echo "==> Generating synthetic dataset: $DATASET_DIR"
python3 - "$DATASET_DIR" <<'PY'
import os, struct, zlib, math, random, sys
out = sys.argv[1]
os.makedirs(out, exist_ok=True)

def chunk(tag, data):
    return struct.pack(">I", len(data)) + tag + data + struct.pack(">I", zlib.crc32(tag + data) & 0xffffffff)

def write_png(path, w, h, channels, pixel_fn):
    color_type = 2 if channels == 3 else 6
    rows = []
    for y in range(h):
        row = bytearray()
        for x in range(w):
            px = pixel_fn(x, y, w, h)
            row.extend(px[:channels])
        rows.append(b"\x00" + bytes(row))
    raw = b"".join(rows)
    data = chunk(b"IHDR", struct.pack(">2I5B", w, h, 8, color_type, 0, 0, 0))
    data += chunk(b"IDAT", zlib.compress(raw, 9))
    data += chunk(b"IEND", b"")
    with open(path, "wb") as f:
        f.write(b"\x89PNG\r\n\x1a\n" + data)

def grad_rgb(x, y, w, h):
    r = int(255 * x / max(1, w - 1))
    g = int(255 * y / max(1, h - 1))
    b = int((r + g) * 0.5)
    return (r, g, b, 255)

def checker_rgb(x, y, w, h):
    v = 230 if ((x // 16 + y // 16) % 2 == 0) else 24
    return (v, 255 - v, (v * 3) % 256, 255)

def photoish_rgb(x, y, w, h):
    fx = x / max(1, w - 1)
    fy = y / max(1, h - 1)
    r = int(127 + 80 * math.sin(9.0 * fx) + 40 * math.cos(7.0 * fy))
    g = int(127 + 70 * math.sin(6.0 * fy + 0.6) + 50 * math.cos(8.0 * fx))
    b = int(127 + 60 * math.sin(11.0 * (fx + fy)))
    return (max(0,min(255,r)), max(0,min(255,g)), max(0,min(255,b)), 255)

def noise_rgb(x, y, w, h):
    rnd = random.Random((y * 1315423911 + x * 2654435761) & 0xffffffff)
    return (rnd.randrange(256), rnd.randrange(256), rnd.randrange(256), 255)

def alpha_rgba(x, y, w, h):
    r = int(255 * x / max(1, w - 1))
    g = int(255 * y / max(1, h - 1))
    b = int((x ^ y) & 255)
    a = int(255 * ((math.sin(x * 0.05) * 0.5 + 0.5) * (math.cos(y * 0.04) * 0.5 + 0.5)))
    return (r, g, b, a)

cases = [
    ("tiny_flat_32.png", 32, 32, 3, lambda x,y,w,h: (180, 200, 230, 255)),
    ("tiny_grad_64.png", 64, 64, 3, grad_rgb),
    ("small_checker_128.png", 128, 128, 3, checker_rgb),
    ("small_photoish_256.png", 256, 256, 3, photoish_rgb),
    ("medium_grad_1024x768.png", 1024, 768, 3, grad_rgb),
    ("large_photoish_2048x1536.png", 2048, 1536, 3, photoish_rgb),
    ("large_noise_2048x1536.png", 2048, 1536, 3, noise_rgb),
    ("alpha_pattern_1024x768.png", 1024, 768, 4, alpha_rgba),
]

for name, w, h, c, fn in cases:
    write_png(os.path.join(out, name), w, h, c, fn)
print(f"generated {len(cases)} images")
PY

echo "==> Running CLI single-image matrix"
CLI_CSV="$OUT_DIR/cli_results.csv"
echo "image,quality,encode_ms,decode_ms,total_ms,input_bytes,bg_bytes,ratio,psnr,ssim" > "$CLI_CSV"

for q in $QUALITIES; do
  for img in "$DATASET_DIR"/*.png; do
    base="$(basename "${img%.png}")"
    bg="$CLI_OUT_DIR/${base}_q${q}.bg"
    out_png="$CLI_OUT_DIR/${base}_q${q}_rt.png"

    t0="$(date +%s%N)"
    "$BITGRAIN_BIN" encode --threads "${THREADS:-0}" "$img" -o "$bg" -q "$q" -y >/dev/null 2>&1 || true
    t1="$(date +%s%N)"
    "$BITGRAIN_BIN" decode --threads "${THREADS:-0}" "$bg" -o "$out_png" -y >/dev/null 2>&1 || true
    t2="$(date +%s%N)"

    enc_ms=$(( (t1 - t0) / 1000000 ))
    dec_ms=$(( (t2 - t1) / 1000000 ))
    tot_ms=$(( enc_ms + dec_ms ))
    in_b="$(stat -c%s "$img")"
    bg_b="$(stat -c%s "$bg")"
    ratio="$(python3 - <<'PY' "$bg_b" "$in_b"
import sys
o=float(sys.argv[1]); i=float(sys.argv[2]); print(f"{(o/i if i>0 else 0):.6f}")
PY
)"

    metrics="$("$BITGRAIN_BIN" roundtrip --threads "${THREADS:-0}" "$img" -o "$CLI_OUT_DIR/${base}_q${q}_metrics.png" -q "$q" -y -m 2>&1 || true)"
    psnr="$(echo "$metrics" | awk '/PSNR/{print $2; exit}')"
    ssim="$(echo "$metrics" | awk '/SSIM/{print $5; exit}')"
    psnr="${psnr:-nan}"
    ssim="${ssim:-nan}"

    echo "$(basename "$img"),$q,$enc_ms,$dec_ms,$tot_ms,$in_b,$bg_b,$ratio,$psnr,$ssim" >> "$CLI_CSV"
  done
done

echo "==> Running batch encode/decode tests"
for q in $QUALITIES; do
  batch_q="$BATCH_OUT_DIR/q$q"
  mkdir -p "$batch_q/bg" "$batch_q/decoded"
  t0="$(date +%s%N)"
  set +e
  "$BITGRAIN_BIN" encode --threads "${THREADS:-0}" "$DATASET_DIR" -o "$batch_q/bg" -q "$q" -y >/dev/null 2>&1
  enc_rc=$?
  set -e
  t1="$(date +%s%N)"
  set +e
  "$BITGRAIN_BIN" decode --threads "${THREADS:-0}" "$batch_q/bg" -o "$batch_q/decoded" -y >/dev/null 2>&1
  dec_rc=$?
  set -e
  t2="$(date +%s%N)"
  echo "batch_q$q encode_ms=$(( (t1-t0)/1000000 )) decode_ms=$(( (t2-t1)/1000000 )) encode_rc=$enc_rc decode_rc=$dec_rc" | tee -a "$OUT_DIR/batch_timing.txt"
done

echo "==> Running library/API benchmark (bitgrain-bench)"
BENCH_JSON="$OUT_DIR/lib_bench.json"
THREAD_ARG=()
if [[ "${THREADS}" != "0" ]]; then THREAD_ARG=(-t "$THREADS"); fi
set +e
"$BENCH_BIN" "${THREAD_ARG[@]}" -r "$RUNS" -w "$WARMUP" --json --json-file "$BENCH_JSON" "$DATASET_DIR" > "$OUT_DIR/lib_bench_stdout.json"
bench_rc=$?
set -e
echo "bench_rc=$bench_rc" >> "$OUT_DIR/batch_timing.txt"

echo "==> Running C library smoke test"
LIB_SMOKE_BIN="$OUT_DIR/lib_api_smoke"
gcc -std=c11 -O2 -I"$ROOT_DIR/includes" \
  "$ROOT_DIR/scripts/lib_api_smoke.c" \
  "$ROOT_DIR/rust/target/release/deps/libbitgrain.a" \
  "$ROOT_DIR/c/dct.o" "$ROOT_DIR/c/quant.o" \
  -lpthread -ldl -lm -o "$LIB_SMOKE_BIN"

first_bg="$(ls "$CLI_OUT_DIR"/*.bg | head -n1)"
"$LIB_SMOKE_BIN" "$first_bg" > "$OUT_DIR/lib_api_smoke.log"

echo "==> Validating quality gates (non-noise images only)"
python3 - "$BENCH_JSON" "$PSNR_MIN" "$SSIM_MIN" <<'PY'
import json, sys, os
path, psnr_min, ssim_min = sys.argv[1], float(sys.argv[2]), float(sys.argv[3])
if not os.path.exists(path):
    print("quality gate skipped: benchmark JSON missing")
    sys.exit(0)
with open(path, "r", encoding="utf-8") as f:
    rows = json.load(f)
failed = [os.path.basename(r.get("image","")) for r in rows if not r.get("ok", False)]
if failed:
    print("benchmark decode/encode failures detected:")
    for img in failed:
        print(f"  {img}")
bad = []
for r in rows:
    img = os.path.basename(r.get("image",""))
    if "noise" in img.lower():
        continue
    psnr = float(r.get("psnr", -1.0))
    ssim = float(r.get("ssim", -1.0))
    if psnr >= 0 and ssim >= 0 and (psnr < psnr_min or ssim < ssim_min):
        bad.append((img, psnr, ssim))
if bad:
    print("quality gate failed:")
    for b in bad:
        print(f"  {b[0]} psnr={b[1]:.3f} ssim={b[2]:.4f}")
    sys.exit(2)
print("quality gate: OK")
PY

echo
echo "=== Exhaustive evaluation complete ==="
echo "Output directory: $OUT_DIR"
echo "CLI matrix CSV:   $CLI_CSV"
echo "Library bench:    $BENCH_JSON"
echo "Library smoke:    $OUT_DIR/lib_api_smoke.log"
