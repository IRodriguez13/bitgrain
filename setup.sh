#!/bin/sh
# Bitgrain setup: install build deps and compile the binary.
# Usage: ./setup.sh   (or: sh setup.sh)

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR"

echo "=== Bitgrain setup ==="

# --- C compiler ---
if command -v gcc >/dev/null 2>&1; then
    echo "[ok] gcc: $(gcc --version | head -1)"
elif command -v cc >/dev/null 2>&1; then
    echo "[ok] cc (will use for build)"
else
    echo "[install] C compiler..."
    if command -v apt-get >/dev/null 2>&1; then
        sudo apt-get update -qq
        sudo apt-get install -y build-essential
    elif command -v dnf >/dev/null 2>&1; then
        sudo dnf install -y gcc make
    elif command -v yum >/dev/null 2>&1; then
        sudo yum install -y gcc make
    elif command -v brew >/dev/null 2>&1; then
        xcode-select -p >/dev/null 2>&1 || xcode-select --install
    else
        echo "Please install gcc and make, then run this script again."
        exit 1
    fi
fi

# Prefer gcc for Makefile
export CC="${CC:-gcc}"
if ! command -v "$CC" >/dev/null 2>&1; then
    export CC=cc
fi

# --- Make ---
if command -v make >/dev/null 2>&1; then
    echo "[ok] make: $(make --version | head -1)"
else
    echo "[install] make..."
    if command -v apt-get >/dev/null 2>&1; then
        sudo apt-get install -y make
    elif command -v dnf >/dev/null 2>&1; then
        sudo dnf install -y make
    elif command -v brew >/dev/null 2>&1; then
        brew install make
    else
        echo "Please install make and run this script again."
        exit 1
    fi
fi

# --- libwebp (required for build) ---
echo "[check] libwebp..."
if pkg-config --exists libwebp 2>/dev/null; then
    echo "[ok] libwebp: $(pkg-config --modversion libwebp)"
else
    echo "[install] libwebp..."
    if command -v apt-get >/dev/null 2>&1; then
        sudo apt-get update -qq
        sudo apt-get install -y libwebp-dev
    elif command -v dnf >/dev/null 2>&1; then
        sudo dnf install -y libwebp-devel
    elif command -v yum >/dev/null 2>&1; then
        sudo yum install -y libwebp-devel
    elif command -v brew >/dev/null 2>&1; then
        brew install webp
    else
        echo "ERROR: libwebp not found. Install manually:"
        echo "  Debian/Ubuntu: sudo apt install libwebp-dev"
        echo "  Fedora/RHEL:   sudo dnf install libwebp-devel"
        echo "  macOS:         brew install webp"
        exit 1
    fi
fi

# --- Rust ---
if command -v rustup >/dev/null 2>&1; then
    # Ensure default toolchain is set (fixes "no default configured")
    echo "[rust] Ensuring default toolchain (stable)..."
    rustup default stable
fi
if command -v cargo >/dev/null 2>&1 && command -v rustc >/dev/null 2>&1; then
    echo "[ok] rustc: $(rustc --version)"
else
    echo "[install] Rust (rustup)..."
    if command -v rustup >/dev/null 2>&1; then
        rustup default stable
    else
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
        export PATH="$HOME/.cargo/bin:$PATH"
    fi
    if ! command -v cargo >/dev/null 2>&1; then
        export PATH="$HOME/.cargo/bin:$PATH"
    fi
fi

# Ensure Rust in PATH for this script
if ! command -v cargo >/dev/null 2>&1; then
    [ -f "$HOME/.cargo/env" ] && . "$HOME/.cargo/env"
    export PATH="$HOME/.cargo/bin:$PATH"
fi
if ! command -v cargo >/dev/null 2>&1; then
    echo "Rust (cargo) not found in PATH. Add ~/.cargo/bin to PATH and run: make"
    exit 1
fi

# --- Build ---
echo ""
echo "=== Building bitgrain ==="
make clean 2>/dev/null || true
make bitgrain

# --- Test ---
if [ -f "./bitgrain" ]; then
    echo ""
    echo "=== Test ==="
    ./bitgrain -v
    ./bitgrain -h | head -5
    echo ""
    # Prueba con imágenes de test_images (o /mnt/c/imagenes_test en WSL)
    IMG_DIR="$SCRIPT_DIR/test_images"
    if [ ! -d "$IMG_DIR" ] || [ -z "$(ls -A "$IMG_DIR" 2>/dev/null)" ]; then
        [ -d "/mnt/c/imagenes_test" ] && IMG_DIR="/mnt/c/imagenes_test"
    fi
    if [ -d "$IMG_DIR" ] && [ -n "$(ls -A "$IMG_DIR" 2>/dev/null)" ]; then
        echo "=== Prueba con $IMG_DIR ==="
        mkdir -p "$SCRIPT_DIR/test_out"
        ./bitgrain -cd -i "$IMG_DIR" -o "$SCRIPT_DIR/test_out" -y -m || true
        echo "Salida en: $SCRIPT_DIR/test_out"
    fi
    echo ""
    echo "Done. Binary: $SCRIPT_DIR/bitgrain"
    echo "Run: ./bitgrain -v   or   ./bitgrain -cd -i <image> -o out.jpg"
else
    echo "Build did not produce ./bitgrain"
    exit 1
fi
