# ==============================
# Toolchains
# ==============================

CC      = gcc
RUST_DIR = rust
RUST_TARGET = $(RUST_DIR)/target/release/libbitgrain.a
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
	cd $(RUST_DIR) && RUSTFLAGS="$(RUSTFLAGS_NATIVE)" cargo build --release

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

.PHONY: all build c clean install rebuild lib-shared

# ==============================
# Clean
# ==============================

clean:
	rm -f $(C_OBJS) c/webp_io.o $(TARGET)
	cd $(RUST_DIR) && cargo clean

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
