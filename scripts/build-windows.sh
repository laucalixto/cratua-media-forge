#!/usr/bin/env bash
# Cross-compile MediaForge for Windows from Linux/WSL with bundled ffmpeg
# Requires: mingw-w64, rustup target add x86_64-pc-windows-gnu
set -euo pipefail

cd "$(dirname "$0")/.."

# Download ffmpeg if not present
if [ ! -f "vendor/ffmpeg/ffmpeg.exe" ]; then
    echo "=== Downloading ffmpeg ==="
    bash scripts/download-ffmpeg.sh
fi

echo "=== Cross-compiling MediaForge for Windows ==="

# Check for mingw
if ! command -v x86_64-w64-mingw32-gcc &>/dev/null; then
    echo "ERROR: mingw-w64 not found. Install with:"
    echo "  sudo apt install mingw-w64"
    exit 1
fi

# Check for Rust target
if ! rustup target list --installed | grep -q x86_64-pc-windows-gnu; then
    echo "Adding Rust target x86_64-pc-windows-gnu..."
    rustup target add x86_64-pc-windows-gnu
fi

# Build
cargo build --release --target x86_64-pc-windows-gnu

BIN="target/x86_64-pc-windows-gnu/release/mediaforge.exe"
if [ -f "$BIN" ]; then
    echo "Binary built: $BIN"
    ls -lh "$BIN"
    x86_64-w64-mingw32-strip "$BIN" 2>/dev/null || true
    ls -lh "$BIN"
else
    echo "ERROR: Binary not found!"
    exit 1
fi

# Create dist directory
DIST="dist/mediaforge-windows"
rm -rf "$DIST"
mkdir -p "$DIST"

cp "$BIN" "$DIST/mediaforge.exe"
mkdir -p "$DIST/ffmpeg"
cp vendor/ffmpeg/ffmpeg.exe "$DIST/ffmpeg/ffmpeg.exe"
cp README.md "$DIST/" 2>/dev/null || echo "# MediaForge" > "$DIST/README.md"

echo ""
echo "=== Cross-compilation complete ==="
echo "Dist: $DIST/"
ls -lh "$DIST/"
echo ""
echo "Distribute the entire dist/mediaforge-windows/ folder."
echo "Users just need to run mediaforge.exe — ffmpeg.exe is auto-detected."
