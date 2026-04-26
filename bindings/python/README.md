# Bitgrain Python bindings

Requires Bitgrain shared libraries. From the **repo root**:

```bash
make install PREFIX=$HOME/.local
```

Optionally set the library path:

```bash
export BITGRAIN_LIB=$HOME/.local/lib/libbitgrain.so
export BITGRAIN_SIMD_LIB=$HOME/.local/lib/libbitgrain-simd.so
```

## Use

```python
import sys
sys.path.insert(0, "/path/to/bitgrain/bindings/python")
import bitgrain

from PIL import Image
im = Image.open("image.png").convert("RGB")
rgb_bytes = im.tobytes()
buf, size = bitgrain.encode_rgb(rgb_bytes, im.width, im.height, quality=85)

# Decode .bg
# pixels, w, h, ch = bitgrain.decode(bg_bytes)
```

Load/save image files (PNG, WebP, JPEG) with Pillow; pass raw bytes to `encode_rgb` / `encode_rgba` / `encode_grayscale` and use `decode()` for .bg streams.
