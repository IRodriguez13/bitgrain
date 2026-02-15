"""
Bitgrain Python bindings (ctypes). Encode/decode .bg image streams.

Build the shared library first (from repo root):
  make bitgrain   # produces rust/target/release/libbitgrain.so or .dylib

Usage:
  import bitgrain
  buf, size = bitgrain.encode_rgb(rgb_bytes, width, height, quality=85)
  pixels, w, h, ch = bitgrain.decode(buf)
"""

import ctypes
import os
import sys

# Load shared lib: prefer env, then same dir, then rust target
_LIB = None
for name in ("BITGRAIN_LIB", "LIBBITGRAIN_SO"):
    path = os.environ.get(name)
    if path and os.path.isfile(path):
        _LIB = ctypes.CDLL(path)
        break
if _LIB is None:
    _base = os.path.dirname(os.path.abspath(__file__))
    for libname in ("libbitgrain.so", "libbitgrain.dylib", "bitgrain.dll"):
        path = os.path.join(_base, libname)
        if os.path.isfile(path):
            _LIB = ctypes.CDLL(path)
            break
if _LIB is None:
    # Try rust/target/release relative to repo root
    _root = os.path.dirname(os.path.dirname(os.path.dirname(_base)))
    _rust = os.path.join(_root, "rust", "target", "release")
    if sys.platform == "darwin":
        path = os.path.join(_rust, "libbitgrain.dylib")
    elif sys.platform == "win32":
        path = os.path.join(_rust, "bitgrain.dll")
    else:
        path = os.path.join(_rust, "libbitgrain.so")
    if os.path.isfile(path):
        _LIB = ctypes.CDLL(path)

if _LIB is None:
    raise OSError(
        "Bitgrain shared library not found. Set BITGRAIN_LIB or build with: make bitgrain"
    )

# Types
_c_int32 = ctypes.c_int32
_c_uint32 = ctypes.c_uint32
_c_uint8 = ctypes.c_uint8
_c_void_p = ctypes.c_void_p

def _setup(name, restype, argtypes):
    f = getattr(_LIB, name)
    f.restype = restype
    f.argtypes = argtypes
    return f

# Encode grayscale
_encode_grayscale = _setup(
    "bitgrain_encode_grayscale",
    ctypes.c_int,
    [ctypes.POINTER(_c_uint8), _c_uint32, _c_uint32, ctypes.POINTER(_c_uint8), _c_uint32, ctypes.POINTER(_c_int32), _c_uint8],
)

# Encode RGB
_encode_rgb = _setup(
    "bitgrain_encode_rgb",
    ctypes.c_int,
    [ctypes.POINTER(_c_uint8), _c_uint32, _c_uint32, ctypes.POINTER(_c_uint8), _c_uint32, ctypes.POINTER(_c_int32), _c_uint8],
)

# Encode RGBA
_encode_rgba = _setup(
    "bitgrain_encode_rgba",
    ctypes.c_int,
    [ctypes.POINTER(_c_uint8), _c_uint32, _c_uint32, ctypes.POINTER(_c_uint8), _c_uint32, ctypes.POINTER(_c_int32), _c_uint8],
)

# Decode
_decode = _setup(
    "bitgrain_decode",
    ctypes.c_int,
    [ctypes.POINTER(_c_uint8), _c_int32, ctypes.POINTER(_c_uint8), _c_uint32, ctypes.POINTER(_c_uint32), ctypes.POINTER(_c_uint32), ctypes.POINTER(_c_uint32)],
)


def encode_grayscale(image: bytes, width: int, height: int, quality: int = 85):
    """Encode 8-bit grayscale to .bg. Returns (bytes, length) or (None, -1)."""
    cap = max(1024, width * height * 2)
    out = (ctypes.c_uint8 * cap)()
    out_len = ctypes.c_int32(0)
    ok = _encode_grayscale(
        (ctypes.c_uint8 * len(image)).from_buffer_copy(image),
        width, height,
        out, cap, ctypes.byref(out_len),
        quality if quality else 85,
    )
    if ok != 0:
        return None, -1
    return bytes(out[: out_len.value]), out_len.value


def encode_rgb(image: bytes, width: int, height: int, quality: int = 85):
    """Encode RGB (24 bpp) to .bg. Returns (bytes, length) or (None, -1)."""
    cap = max(1024, width * height * 3 * 2)
    out = (ctypes.c_uint8 * cap)()
    out_len = ctypes.c_int32(0)
    ok = _encode_rgb(
        (ctypes.c_uint8 * len(image)).from_buffer_copy(image),
        width, height,
        out, cap, ctypes.byref(out_len),
        quality if quality else 85,
    )
    if ok != 0:
        return None, -1
    return bytes(out[: out_len.value]), out_len.value


def encode_rgba(image: bytes, width: int, height: int, quality: int = 85):
    """Encode RGBA (32 bpp) to .bg. Returns (bytes, length) or (None, -1)."""
    cap = max(1024, width * height * 4 * 2)
    out = (ctypes.c_uint8 * cap)()
    out_len = ctypes.c_int32(0)
    ok = _encode_rgba(
        (ctypes.c_uint8 * len(image)).from_buffer_copy(image),
        width, height,
        out, cap, ctypes.byref(out_len),
        quality if quality else 85,
    )
    if ok != 0:
        return None, -1
    return bytes(out[: out_len.value]), out_len.value


def decode(data: bytes):
    """Decode .bg stream. Returns (pixels_bytes, width, height, channels) or (None, 0, 0, 0)."""
    # Header gives max size; use 4 * w * h upper bound from 12-byte header
    if len(data) < 12:
        return None, 0, 0, 0
    w = int.from_bytes(data[2:6], "little")
    h = int.from_bytes(data[6:10], "little")
    cap = w * h * 4
    if cap == 0:
        return None, 0, 0, 0
    out = (ctypes.c_uint8 * cap)()
    out_w = ctypes.c_uint32(0)
    out_h = ctypes.c_uint32(0)
    out_ch = ctypes.c_uint32(0)
    ok = _decode(
        (ctypes.c_uint8 * len(data)).from_buffer_copy(data),
        len(data),
        out, cap,
        ctypes.byref(out_w), ctypes.byref(out_h), ctypes.byref(out_ch),
    )
    if ok != 0:
        return None, 0, 0, 0
    ch = out_ch.value
    return bytes(out[: out_w.value * out_h.value * ch]), out_w.value, out_h.value, ch
