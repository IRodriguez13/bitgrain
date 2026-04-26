# ==============================
# Toolchains
# ==============================

CC      = gcc
RUST_DIR = rust
# Cargo emits the staticlib under deps/ (not next to libbitgrain.so).
RUST_TARGET = $(RUST_DIR)/target/release/deps/libbitgrain.a
TARGET  = bitgrain
BITGRAIN_VERSION ?= 2.0.0
ABI_MAJOR ?= 2

# Base C flags
CFLAGS  = -std=c11 -Wall -Wextra -Iincludes -Ic
PIC_CFLAGS = -fPIC

# Release optimizations. Portable: make CFLAGS_NATIVE= RUSTFLAGS_NATIVE=
CFLAGS  += -O3 -DNDEBUG
CFLAGS_NATIVE ?= -march=native
RUSTFLAGS_NATIVE ?= -C target-cpu=native
CFLAGS  += $(CFLAGS_NATIVE)

# Aggressive math flags only for compute hot paths.
# Keep this scoped (not global) to avoid unintended behavior changes elsewhere.
HOT_MATH_CFLAGS = -ffast-math -fno-math-errno -fno-trapping-math

# WebP (standard; requires libwebp). pkg-config sets -I and -L when present.
WEBP_CFLAGS  := $(shell pkg-config --cflags libwebp 2>/dev/null)
WEBP_LIBS    := $(shell pkg-config --libs libwebp 2>/dev/null)
ifeq ($(WEBP_LIBS),)
  WEBP_LIBS = -lwebp
endif
CFLAGS  += -DBITGRAIN_USE_WEBP $(WEBP_CFLAGS)

# libpng for ICC profile support (optional)
PNG_CFLAGS   := $(shell pkg-config --cflags libpng 2>/dev/null)
PNG_LIBS     := $(shell pkg-config --libs libpng 2>/dev/null)
ifneq ($(PNG_LIBS),)
  CFLAGS  += -DBITGRAIN_USE_PNG_ICC $(PNG_CFLAGS)
  LDFLAGS_EXTRA += $(PNG_LIBS)
endif

LDFLAGS_EXTRA ?=

# OS-specific libs
UNAME_S := $(shell uname -s 2>/dev/null || echo Unknown)
ifeq ($(UNAME_S),Linux)
  LDFLAGS_EXTRA += -lpthread -ldl -lm $(WEBP_LIBS)
endif
ifeq ($(UNAME_S),Darwin)
  LDFLAGS_EXTRA += -lpthread -ldl -lm $(WEBP_LIBS)
endif
ifeq ($(UNAME_S),Unknown)
  LDFLAGS_EXTRA += -lpthread -ldl -lm $(WEBP_LIBS)
endif
ifneq (,$(findstring MINGW,$(UNAME_S)))
  LDFLAGS_EXTRA += -lpthread -lm $(WEBP_LIBS)
endif
ifneq (,$(findstring MSYS,$(UNAME_S)))
  LDFLAGS_EXTRA += -lpthread -lm $(WEBP_LIBS)
endif

LDFLAGS = $(LDFLAGS_EXTRA)

C_SRCS = \
	c/dct.c \
	c/quant.c \
	c/bg_utils.c \
	c/path_utils.c \
	c/cli.c \
	c/roundtrip_cli.c \
	c/decode_cli.c \
	c/encode_cli.c \
	c/image_loader.c \
	c/image_writer.c \
	c/icc_io.c \
	c/platform.c \
	c/metrics.c \
	c/webp_io.c \
	main.c

C_OBJS = $(C_SRCS:.c=.o)

# ==============================
# Default target
# ==============================

all: build

build: bitgrain

# ==============================
# Full build
# ==============================

bitgrain: $(RUST_TARGET) $(C_OBJS)
	$(CC) $(C_OBJS) $(RUST_TARGET) -o $(TARGET) $(LDFLAGS)
	strip $(TARGET) 2>/dev/null || true

# ==============================
# Rust build
# ==============================

$(RUST_TARGET):
	cd $(RUST_DIR) && CARGO_TARGET_DIR="$(abspath $(RUST_DIR)/target)" RUSTFLAGS="$(RUSTFLAGS_NATIVE)" cargo build --release

# Shared library for Python/Go bindings (Rust cdylib: libbitgrain.so or .dylib)
RUST_SO = $(RUST_DIR)/target/release/libbitgrain.so
ifeq ($(UNAME_S),Darwin)
  RUST_SO = $(RUST_DIR)/target/release/libbitgrain.dylib
endif

LIBSIMD_BASENAME = libbitgrain-simd
LIBSIMD_SONAME = $(LIBSIMD_BASENAME).so.$(ABI_MAJOR)
LIBSIMD_REAL = $(LIBSIMD_BASENAME).so.$(BITGRAIN_VERSION)
LIBSIMD_LINK = $(LIBSIMD_BASENAME).so

LIBBITGRAIN_BASENAME = libbitgrain
LIBBITGRAIN_SONAME = $(LIBBITGRAIN_BASENAME).so.$(ABI_MAJOR)
LIBBITGRAIN_REAL = $(LIBBITGRAIN_BASENAME).so.$(BITGRAIN_VERSION)
LIBBITGRAIN_LINK = $(LIBBITGRAIN_BASENAME).so

ifeq ($(UNAME_S),Darwin)
  LIBSIMD_SONAME = $(LIBSIMD_BASENAME).$(ABI_MAJOR).dylib
  LIBSIMD_REAL = $(LIBSIMD_BASENAME).$(BITGRAIN_VERSION).dylib
  LIBSIMD_LINK = $(LIBSIMD_BASENAME).dylib
  LIBBITGRAIN_SONAME = $(LIBBITGRAIN_BASENAME).$(ABI_MAJOR).dylib
  LIBBITGRAIN_REAL = $(LIBBITGRAIN_BASENAME).$(BITGRAIN_VERSION).dylib
  LIBBITGRAIN_LINK = $(LIBBITGRAIN_BASENAME).dylib
endif

BUILD_LIB_DIR = build/lib
PIC_DCT_OBJ = $(BUILD_LIB_DIR)/dct.pic.o
PIC_QUANT_OBJ = $(BUILD_LIB_DIR)/quant.pic.o
LIBSIMD_PATH = $(BUILD_LIB_DIR)/$(LIBSIMD_REAL)
LIBBITGRAIN_PATH = $(BUILD_LIB_DIR)/$(LIBBITGRAIN_REAL)

lib-shared: $(RUST_TARGET)
	@test -f $(RUST_SO) && echo "Shared lib: $(RUST_SO)" || true

libsimd: $(LIBSIMD_PATH)
	@ln -sfn $(LIBSIMD_REAL) $(BUILD_LIB_DIR)/$(LIBSIMD_SONAME)
	@ln -sfn $(LIBSIMD_SONAME) $(BUILD_LIB_DIR)/$(LIBSIMD_LINK)
	@echo "Built: $(BUILD_LIB_DIR)/$(LIBSIMD_LINK)"

libbitgrain-shared: $(LIBBITGRAIN_PATH)
	@ln -sfn $(LIBBITGRAIN_REAL) $(BUILD_LIB_DIR)/$(LIBBITGRAIN_SONAME)
	@ln -sfn $(LIBBITGRAIN_SONAME) $(BUILD_LIB_DIR)/$(LIBBITGRAIN_LINK)
	@echo "Built: $(BUILD_LIB_DIR)/$(LIBBITGRAIN_LINK)"

# ==============================
# C build
# ==============================

c: $(C_OBJS)

%.o: %.c
	$(CC) $(CFLAGS) -c $< -o $@

c/dct.o: c/dct.c
	$(CC) $(CFLAGS) $(HOT_MATH_CFLAGS) -c $< -o $@

c/quant.o: c/quant.c
	$(CC) $(CFLAGS) $(HOT_MATH_CFLAGS) -c $< -o $@

$(PIC_DCT_OBJ): c/dct.c
	@mkdir -p $(BUILD_LIB_DIR)
	$(CC) $(CFLAGS) $(PIC_CFLAGS) $(HOT_MATH_CFLAGS) -c $< -o $@

$(PIC_QUANT_OBJ): c/quant.c
	@mkdir -p $(BUILD_LIB_DIR)
	$(CC) $(CFLAGS) $(PIC_CFLAGS) $(HOT_MATH_CFLAGS) -c $< -o $@

$(LIBSIMD_PATH): $(PIC_DCT_OBJ) $(PIC_QUANT_OBJ)
ifeq ($(UNAME_S),Darwin)
	$(CC) -dynamiclib -Wl,-install_name,@rpath/$(LIBSIMD_SONAME) -Wl,-compatibility_version,$(ABI_MAJOR) -Wl,-current_version,$(BITGRAIN_VERSION) -o $@ $^ -lm
else
	$(CC) -shared -Wl,-soname,$(LIBSIMD_SONAME) -o $@ $^ -lm
endif

$(LIBBITGRAIN_PATH): $(RUST_TARGET) $(PIC_DCT_OBJ) $(PIC_QUANT_OBJ)
ifeq ($(UNAME_S),Darwin)
	$(CC) -dynamiclib -Wl,-install_name,@rpath/$(LIBBITGRAIN_SONAME) -Wl,-compatibility_version,$(ABI_MAJOR) -Wl,-current_version,$(BITGRAIN_VERSION) -o $@ -Wl,-all_load $(RUST_TARGET) $(PIC_DCT_OBJ) $(PIC_QUANT_OBJ) -lpthread -ldl -lm
else
	$(CC) -shared -Wl,-soname,$(LIBBITGRAIN_SONAME) -Wl,--whole-archive $(RUST_TARGET) -Wl,--no-whole-archive $(PIC_DCT_OBJ) $(PIC_QUANT_OBJ) -o $@ -lpthread -ldl -lm
endif

main.o: main.c
	$(CC) $(CFLAGS) -c $< -o $@

.PHONY: all build c bench clean install rebuild lib-shared libsimd libbitgrain-shared \
	build-portable build-avx2 bench-avx2 lib-consumer-smoke

# ==============================
# Bench (standalone profiler)
# ==============================

BENCH_TARGET = bitgrain-bench
BENCH_CFLAGS = -std=c11 -Wall -Wextra -Iincludes -Ic -Ibench -O3 -DNDEBUG $(CFLAGS_NATIVE)
BENCH_SRCS   = bench/bench.c bench/main.c c/dct.c c/quant.c c/metrics.c
BENCH_OBJS   = $(BENCH_SRCS:.c=.o)

bench: $(RUST_TARGET) $(BENCH_OBJS)
	$(CC) $(BENCH_OBJS) $(RUST_TARGET) -o $(BENCH_TARGET) $(LDFLAGS)
	@echo "Built: $(BENCH_TARGET)"

bench/bench.o: bench/bench.c bench/bench.h
	$(CC) $(BENCH_CFLAGS) -c $< -o $@

bench/main.o: bench/main.c bench/bench.h
	$(CC) $(BENCH_CFLAGS) -c $< -o $@

# ==============================
# Clean
# ==============================

clean:
	rm -f $(C_OBJS) c/webp_io.o $(TARGET) $(BENCH_TARGET) $(BENCH_OBJS)
	rm -rf $(BUILD_LIB_DIR) build/pkgconfig build/cmake
	cd $(RUST_DIR) && CARGO_TARGET_DIR="$(abspath $(RUST_DIR)/target)" cargo clean

# ==============================
# Install (C library + CLI)
# ==============================
# Usage: make install [PREFIX=/usr/local]
PREFIX ?= /usr/local

install: bitgrain libbitgrain-shared libsimd
	install -d $(DESTDIR)$(PREFIX)/bin
	install -m 755 bitgrain $(DESTDIR)$(PREFIX)/bin/
	install -d $(DESTDIR)$(PREFIX)/include/bitgrain
	install -m 644 includes/encoder.h $(DESTDIR)$(PREFIX)/include/bitgrain/
	install -d $(DESTDIR)$(PREFIX)/lib
	install -m 644 $(RUST_TARGET) $(DESTDIR)$(PREFIX)/lib/libbitgrain.a
	install -m 755 $(LIBBITGRAIN_PATH) $(DESTDIR)$(PREFIX)/lib/$(LIBBITGRAIN_REAL)
	ln -sfn $(LIBBITGRAIN_REAL) $(DESTDIR)$(PREFIX)/lib/$(LIBBITGRAIN_SONAME)
	ln -sfn $(LIBBITGRAIN_SONAME) $(DESTDIR)$(PREFIX)/lib/$(LIBBITGRAIN_LINK)
	install -m 755 $(LIBSIMD_PATH) $(DESTDIR)$(PREFIX)/lib/$(LIBSIMD_REAL)
	ln -sfn $(LIBSIMD_REAL) $(DESTDIR)$(PREFIX)/lib/$(LIBSIMD_SONAME)
	ln -sfn $(LIBSIMD_SONAME) $(DESTDIR)$(PREFIX)/lib/$(LIBSIMD_LINK)
	install -d build/pkgconfig
	sed \
		-e 's|@PREFIX@|$(PREFIX)|g' \
		-e 's|@VERSION@|$(BITGRAIN_VERSION)|g' \
		-e 's|@ABI_MAJOR@|$(ABI_MAJOR)|g' \
		pkgconfig/bitgrain.pc.in > build/pkgconfig/bitgrain.pc
	install -d $(DESTDIR)$(PREFIX)/lib/pkgconfig
	install -m 644 build/pkgconfig/bitgrain.pc $(DESTDIR)$(PREFIX)/lib/pkgconfig/bitgrain.pc
	install -d build/cmake
	sed \
		-e 's|@PREFIX@|$(PREFIX)|g' \
		-e 's|@VERSION@|$(BITGRAIN_VERSION)|g' \
		-e 's|@ABI_MAJOR@|$(ABI_MAJOR)|g' \
		cmake/BitgrainConfig.cmake.in > build/cmake/BitgrainConfig.cmake
	install -d $(DESTDIR)$(PREFIX)/lib/cmake/Bitgrain
	install -m 644 build/cmake/BitgrainConfig.cmake $(DESTDIR)$(PREFIX)/lib/cmake/Bitgrain/BitgrainConfig.cmake
	install -d $(DESTDIR)$(PREFIX)/share/bash-completion/completions
	install -m 644 completions/bitgrain.bash $(DESTDIR)$(PREFIX)/share/bash-completion/completions/bitgrain
	install -d $(DESTDIR)$(PREFIX)/share/man/man1
	install -m 644 man/bitgrain.1 $(DESTDIR)$(PREFIX)/share/man/man1/bitgrain.1
	@echo "Installed: CLI + static/shared libs + pkg-config + CMake config"

lib-consumer-smoke: install
	@PKG_CONFIG_PATH="$(DESTDIR)$(PREFIX)/lib/pkgconfig:$$PKG_CONFIG_PATH" \
	$(CC) scripts/lib_api_smoke.c -o /tmp/bitgrain_lib_api_smoke \
	$$(PKG_CONFIG_PATH="$(DESTDIR)$(PREFIX)/lib/pkgconfig:$$PKG_CONFIG_PATH" pkg-config --cflags --libs bitgrain)
	@LD_LIBRARY_PATH="$(DESTDIR)$(PREFIX)/lib:$$LD_LIBRARY_PATH" /tmp/bitgrain_lib_api_smoke /tmp/nonexistent.bg >/dev/null 2>&1 || true

# ==============================
# Rebuild
# ==============================

rebuild: clean build

# ==============================
# CPU-specific optimization presets (opt-in)
# ==============================
# Portable baseline (safe to distribute across CPUs):
#   make build-portable
build-portable:
	$(MAKE) CFLAGS_NATIVE= RUSTFLAGS_NATIVE= build

# x86_64 AVX2/FMA preset (host must support these ISA extensions):
#   make build-avx2
#   make bench-avx2
build-avx2:
	$(MAKE) CFLAGS_NATIVE="-mavx2 -mfma -mbmi2 -mlzcnt" RUSTFLAGS_NATIVE="-C target-feature=+avx2,+fma,+bmi2,+lzcnt" build

bench-avx2:
	$(MAKE) CFLAGS_NATIVE="-mavx2 -mfma -mbmi2 -mlzcnt" RUSTFLAGS_NATIVE="-C target-feature=+avx2,+fma,+bmi2,+lzcnt" bench
