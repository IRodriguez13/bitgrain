#!/bin/sh
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR"
BINARY=bitgrain
DEST=/usr/bin

if [ ! -f "$BINARY" ]; then
	echo "Binary $BINARY not found. Building..."
	make bitgrain
fi

echo "Installing $BINARY to $DEST (requires sudo)"
sudo install -m 755 "$BINARY" "$DEST/$BINARY"
echo "Done. Run: $BINARY -v"
