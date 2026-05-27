#!/usr/bin/env bash
# Build MediaForge for Linux (release) with bundled ffmpeg
set -euo pipefail

cd "$(dirname "$0")/.."

# Download ffmpeg if not present
if [ ! -f "vendor/ffmpeg/ffmpeg" ]; then
    echo "=== Downloading ffmpeg ==="
    bash scripts/download-ffmpeg.sh
fi

echo "=== Building MediaForge for Linux ==="
cargo build --release

BIN="target/release/mediaforge"
if [ -f "$BIN" ]; then
    echo "Binary built: $BIN"
    ls -lh "$BIN"
    strip "$BIN" 2>/dev/null || true
    ls -lh "$BIN"
else
    echo "ERROR: Binary not found!"
    exit 1
fi

# Create dist directory
DIST="dist/mediaforge-linux"
rm -rf "$DIST"
mkdir -p "$DIST"

cp "$BIN" "$DIST/mediaforge"
mkdir -p "$DIST/ffmpeg"
cp vendor/ffmpeg/ffmpeg "$DIST/ffmpeg/ffmpeg"
cp README.md "$DIST/" 2>/dev/null || echo "# MediaForge" > "$DIST/README.md"

echo ""
echo "=== Build complete ==="
echo "Dist: $DIST/"
ls -lh "$DIST/"
echo ""
echo "Run: WINIT_X11_SCALE_FACTOR=2 $DIST/mediaforge  # for HiDPI"
echo ""
echo "To create AppImage (requires appimagetool):"
echo "  See scripts/build-appimage.sh"
