#!/bin/bash
set -e

echo "Building CrabOCR release..."
# Use default target if musl not available locally
cargo build --release

DEST="crabocr-release"
mkdir -p $DEST
mkdir -p $DEST/tessdata

cp target/release/crabocr $DEST/
if [ -d tessdata ]; then
    cp tessdata/* $DEST/tessdata/ 2>/dev/null || true
fi
cp README.md $DEST/ 2>/dev/null || true
cp LICENSE $DEST/ 2>/dev/null || true

tar -czf crabocr-linux-gnu.tar.gz $DEST
echo "Release created: crabocr-linux-gnu.tar.gz"
