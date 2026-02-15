# Bitgrain Python bindings

Requires the Bitgrain shared library (Rust cdylib). From the **repo root**:

```bash
make bitgrain   # builds rust/target/release/libbitgrain.so (or .dylib on macOS)
```

Optionally set the library path:

```bash
export BITGRAIN_LIB=/path/to/bitgrain/rust/target/release/libbitgrain.so
```

## Use

```python
import sys
sys.path.insert(0, "/path/to/bitgrain/bindings/python")
import bitgrain

# Encode RGB (e.g. from Pillow: image.tobytes(), mode "RGB")
rgb = open("image.png", "rb")  # use Pillow to load and convert to RGB bytes
# buf, size = bitgrain.encode_rgb(rgb_bytes, width, height, quality=85)

# Decode .bg
# pixels, w, h, ch = bitgrain.decode(bg_bytes)
```

Load/save image files (PNG, WebP, JPEG) with Pillow; pass raw bytes to `encode_rgb` / `encode_rgba` / `encode_grayscale` and use `decode()` for .bg streams.
