#!/usr/bin/env bash
# Build Cratua Media Forge for Linux (Tauri) — portable archive
set -euo pipefail
cd "$(dirname "$0")/.."

echo "=== Building Cratua Media Forge for Linux ==="

# 1. Build via Tauri CLI (compiles Rust + bundles .deb/.rpm)
echo "[1/3] Building with Tauri..."
cd crates/mediaforge-tauri
cargo tauri build 2>&1 | grep -E "Finished|Bundling|Error|Built"
cd ../..

BIN="target/release/mediaforge-tauri"
if [ ! -f "$BIN" ]; then
    echo "ERROR: Binary not found at $BIN"
    exit 1
fi
echo "  Binary: $(ls -lh "$BIN" | awk '{print $5}')"

# 2. Create portable dist
echo "[2/3] Creating portable package..."
DIST="dist/cratua-media-forge-linux"
rm -rf "$DIST"
mkdir -p "$DIST/ffmpeg"
cp "$BIN" "$DIST/cratua-media-forge"
cp vendor/ffmpeg/ffmpeg "$DIST/ffmpeg/"

cat > "$DIST/README.txt" << 'EOF'
Cratua Media Forge — Portable Media Converter
===============================================

Run:
  ./cratua-media-forge

For HiDPI monitors:
  GDK_DPI_SCALE=2 ./cratua-media-forge

ffmpeg is bundled — no installation needed.
EOF

# 3. Create archive
echo "[3/3] Creating archive..."
cd dist
tar czf "cratua-media-forge-v0.1.0-linux-x86_64.tar.gz" cratua-media-forge-linux
cd ..

echo ""
echo "=== Build complete ==="
echo "  Portable: $(ls -lh dist/cratua-media-forge-v0.1.0-linux-x86_64.tar.gz | awk '{print $5}')"
echo "  Deb:      $(ls -lh target/release/bundle/deb/*.deb 2>/dev/null | awk '{print $5}')"
echo "  RPM:      $(ls -lh target/release/bundle/rpm/*.rpm 2>/dev/null | awk '{print $5}')"
