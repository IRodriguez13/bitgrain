# ==============================
# Toolchains
# ==============================

CC      = gcc
RUST_DIR = rust
# Cargo emits the staticlib under deps/ (not next to libbitgrain.so).
RUST_TARGET = $(RUST_DIR)/target/release/deps/libbitgrain.a
TARGET  = bitgrain

# Base C flags
CFLAGS  = -std=c11 -Wall -Wextra -Iincludes -Ic

# Release optimizations. Portable: make CFLAGS_NATIVE= RUSTFLAGS_NATIVE=
CFLAGS  += -O3 -DNDEBUG
CFLAGS_NATIVE ?= -march=native
RUSTFLAGS_NATIVE ?= -C target-cpu=native
CFLAGS  += $(CFLAGS_NATIVE)

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

lib-shared: $(RUST_TARGET)
	@test -f $(RUST_SO) && echo "Shared lib: $(RUST_SO)" || true

# ==============================
# C build
# ==============================

c: $(C_OBJS)

%.o: %.c
	$(CC) $(CFLAGS) -c $< -o $@

main.o: main.c
	$(CC) $(CFLAGS) -c $< -o $@

.PHONY: all build c bench clean install rebuild lib-shared build-portable build-avx2 bench-avx2

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
	cd $(RUST_DIR) && CARGO_TARGET_DIR="$(abspath $(RUST_DIR)/target)" cargo clean

# ==============================
# Install (C library + CLI)
# ==============================
# Usage: make install [PREFIX=/usr/local]
PREFIX ?= /usr/local

install: bitgrain
	install -d $(DESTDIR)$(PREFIX)/bin
	install -m 755 bitgrain $(DESTDIR)$(PREFIX)/bin/
	install -d $(DESTDIR)$(PREFIX)/include/bitgrain
	install -m 644 includes/encoder.h $(DESTDIR)$(PREFIX)/include/bitgrain/
	install -d $(DESTDIR)$(PREFIX)/lib
	install -m 644 $(RUST_TARGET) $(DESTDIR)$(PREFIX)/lib/libbitgrain.a
	@if [ -f $(RUST_SO) ]; then install -m 755 $(RUST_SO) $(DESTDIR)$(PREFIX)/lib/; echo "Installed shared lib: $(PREFIX)/lib/"; fi
	@echo "Installed: $(DESTDIR)$(PREFIX)/bin/bitgrain, include/bitgrain/encoder.h, lib/libbitgrain.a"

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
