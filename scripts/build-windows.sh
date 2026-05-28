#!/usr/bin/env bash
# Cross-compile Cratua Media Forge for Windows via Tauri CLI
set -euo pipefail
cd "$(dirname "$0")/.."

echo "=== Cross-compiling Cratua Media Forge for Windows ==="

# Check prerequisites
if ! command -v x86_64-w64-mingw32-gcc &>/dev/null; then
    echo "ERROR: mingw-w64 not found. Install: sudo apt install mingw-w64"
    exit 1
fi

# 1. Build via Tauri CLI (cross-compile)
echo "[1/3] Cross-compiling with Tauri..."
cd crates/mediaforge-tauri

# Ensure target is installed
rustup target add x86_64-pc-windows-gnu 2>/dev/null || true

# Tauri build for Windows target
cargo tauri build --target x86_64-pc-windows-gnu 2>&1 | grep -E "Finished|Bundling|Error|Built|Info"
cd ../..

BIN="target/x86_64-pc-windows-gnu/release/mediaforge-tauri.exe"
if [ ! -f "$BIN" ]; then
    echo "ERROR: Binary not found at $BIN"
    exit 1
fi
echo "  Binary: $(ls -lh "$BIN" | awk '{print $5}')"

# 2. Create portable dist
echo "[2/3] Creating portable package..."
DIST="dist/cratua-media-forge-windows"
rm -rf "$DIST"
mkdir -p "$DIST/ffmpeg"
cp "$BIN" "$DIST/cratua-media-forge.exe"
cp vendor/ffmpeg/ffmpeg.exe "$DIST/ffmpeg/"

# Copy WebView2Loader if Tauri bundled it (MUST be next to .exe — Windows
# does not search subdirectories for DLLs at load time)
WEBVIEW2_DLL="target/x86_64-pc-windows-gnu/release/WebView2Loader.dll"
if [ -f "$WEBVIEW2_DLL" ]; then
    cp "$WEBVIEW2_DLL" "$DIST/"
    echo "  Bundled WebView2Loader.dll"
else
    echo "  NOTE: WebView2Loader.dll not found — Windows 10+ has it built-in."
fi

cat > "$DIST/README.txt" << 'EOF'
Cratua Media Forge — Portable Media Converter
===============================================

Run:
  cratua-media-forge.exe

Requires: Windows 10+ (WebView2 is built-in) or install:
  https://go.microsoft.com/fwlink/p/?LinkId=2124703

ffmpeg is bundled — no installation needed.
Just extract and run.
EOF

# 3. Create archive
echo "[3/3] Creating archive..."
cd dist
zip -r "cratua-media-forge-v0.1.0-windows-x86_64.zip" cratua-media-forge-windows
cd ..

echo ""
echo "=== Build complete ==="
echo "  Portable: $(ls -lh dist/cratua-media-forge-v0.1.0-windows-x86_64.zip | awk '{print $5}')"
