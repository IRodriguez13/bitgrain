#!/bin/sh
set -e

DEST=/usr/bin
BINARY=bitgrain
MAN_DEST_DIR=/usr/share/man/man1
MAN_DEST_NAME=bitgrain.1
COMPL_BASH_DEST_DIR=/usr/share/bash-completion/completions
COMPL_BASH_DEST_NAME=bitgrain
COMPL_BASH_LEGACY_DIR=/etc/bash_completion.d

if [ -f "$DEST/$BINARY" ]; then
	echo "Removing $DEST/$BINARY (requires sudo)"
	sudo rm -f "$DEST/$BINARY"
else
	echo "$DEST/$BINARY not found. Skipping binary removal."
fi

if [ -f "$MAN_DEST_DIR/$MAN_DEST_NAME" ]; then
	echo "Removing man page $MAN_DEST_DIR/$MAN_DEST_NAME (requires sudo)"
	sudo rm -f "$MAN_DEST_DIR/$MAN_DEST_NAME"
fi

if [ -f "$COMPL_BASH_DEST_DIR/$COMPL_BASH_DEST_NAME" ]; then
	echo "Removing bash completion $COMPL_BASH_DEST_DIR/$COMPL_BASH_DEST_NAME (requires sudo)"
	sudo rm -f "$COMPL_BASH_DEST_DIR/$COMPL_BASH_DEST_NAME"
fi

if [ -f "$COMPL_BASH_LEGACY_DIR/$COMPL_BASH_DEST_NAME" ]; then
	echo "Removing bash completion $COMPL_BASH_LEGACY_DIR/$COMPL_BASH_DEST_NAME (requires sudo)"
	sudo rm -f "$COMPL_BASH_LEGACY_DIR/$COMPL_BASH_DEST_NAME"
fi
echo "\nDone."
echo "Thank you for using Bitgrain."
