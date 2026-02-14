# ==============================
# Toolchains
# ==============================

CC      = gcc
CFLAGS  = -std=c11 -Wall -Wextra -Iincludes -Ic

RUST_DIR = rust
RUST_TARGET = $(RUST_DIR)/target/release/libbitgrain.a

C_SRCS = \
	c/quant.c \
	c/bitstream.c \
	c/image_loader.c \
	main.c

C_OBJS = $(C_SRCS:.c=.o)

TARGET = bitgrain


# ==============================
# Default target
# ==============================

all: build


# ==============================
# Full build
# ==============================

bitgrain: $(RUST_TARGET) c
	$(CC) $(C_OBJS) $(RUST_TARGET) -o $(TARGET) -lpthread -ldl -lm


# ==============================
# Rust build (el .a depende de Cargo, así make siempre compila si falta)
# ==============================

$(RUST_TARGET):
	cd $(RUST_DIR) && cargo build --release


# ==============================
# C build
# ==============================

c: $(C_OBJS)

%.o: %.c
	$(CC) $(CFLAGS) -c $< -o $@


.PHONY: all build c clean rebuild

# ==============================
# Clean
# ==============================

clean:
	rm -f $(C_OBJS) $(TARGET)
	cd $(RUST_DIR) && cargo clean


# ==============================
# Rebuild
# ==============================

rebuild: clean build
