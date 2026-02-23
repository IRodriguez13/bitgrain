#!/bin/sh
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR"
BINARY=bitgrain
DEST=/usr/bin
MAN_SRC="man/bitgrain.1"
MAN_DEST_DIR=/usr/share/man/man1
MAN_DEST_NAME=bitgrain.1
COMPL_BASH_SRC="completions/bitgrain.bash"
COMPL_BASH_DEST_DIR=/usr/share/bash-completion/completions
COMPL_BASH_DEST_NAME=bitgrain
COMPL_BASH_LEGACY_DIR=/etc/bash_completion.d

if [ ! -f "$BINARY" ]; then
	echo "Binary $BINARY not found. Building..."
	make bitgrain
fi

echo "Installing $BINARY to $DEST (requires sudo)"
sudo install -m 755 "$BINARY" "$DEST/$BINARY"

if [ -f "$MAN_SRC" ]; then
	echo "Installing man page to $MAN_DEST_DIR (requires sudo)"
	sudo install -d "$MAN_DEST_DIR"
	sudo install -m 644 "$MAN_SRC" "$MAN_DEST_DIR/$MAN_DEST_NAME"
fi

if [ -f "$COMPL_BASH_SRC" ]; then
	echo "Installing bash completion to $COMPL_BASH_DEST_DIR (requires sudo)"
	sudo install -d "$COMPL_BASH_DEST_DIR"
	sudo install -m 644 "$COMPL_BASH_SRC" "$COMPL_BASH_DEST_DIR/$COMPL_BASH_DEST_NAME"
	if [ -d "$COMPL_BASH_LEGACY_DIR" ]; then
		echo "Installing bash completion to $COMPL_BASH_LEGACY_DIR (requires sudo)"
		sudo install -m 644 "$COMPL_BASH_SRC" "$COMPL_BASH_LEGACY_DIR/$COMPL_BASH_DEST_NAME"
	fi
fi
echo "Done. Run: $BINARY -v"
