#!/bin/sh
set -e

DEST=/usr/bin
BINARY=bitgrain

if [ ! -f "$DEST/$BINARY" ]; then
	echo "$DEST/$BINARY not found. Nothing to uninstall."
	exit 0
fi

echo "Removing $DEST/$BINARY (requires sudo)"
sudo rm -f "$DEST/$BINARY"
echo "\nDone."
echo "Thank you for using Bitgrain."
