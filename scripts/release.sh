#!/usr/bin/env bash
# Release: build + bundle for Linux and Windows
set -euo pipefail

cd "$(dirname "$0")/.."

VERSION="${1:-0.1.0}"
echo "=== MediaForge Release v${VERSION} ==="

# Ensure ffmpeg is downloaded
if [ ! -f "vendor/ffmpeg/ffmpeg" ] || [ ! -f "vendor/ffmpeg/ffmpeg.exe" ]; then
    echo "Downloading ffmpeg..."
    bash scripts/download-ffmpeg.sh
fi

rm -rf dist
mkdir -p dist

# ── Linux ──
echo ""
echo "=== Building Linux release ==="
cargo build --release
strip target/release/mediaforge 2>/dev/null || true

mkdir -p dist/mediaforge-linux
cp target/release/mediaforge dist/mediaforge-linux/
mkdir -p dist/mediaforge-linux/ffmpeg
cp vendor/ffmpeg/ffmpeg dist/mediaforge-linux/ffmpeg/
cat > dist/mediaforge-linux/README.txt << 'EOF'
MediaForge - Portable Media Converter
======================================

Run:
  ./mediaforge

For HiDPI monitors:
  WINIT_X11_SCALE_FACTOR=2 ./mediaforge

ffmpeg is bundled — no installation needed.
EOF

cd dist && tar czf "mediaforge-v${VERSION}-linux-x86_64.tar.gz" mediaforge-linux && cd ..
echo "  -> dist/mediaforge-v${VERSION}-linux-x86_64.tar.gz"
ls -lh "dist/mediaforge-v${VERSION}-linux-x86_64.tar.gz"

# ── Windows ──
echo ""
echo "=== Cross-compiling Windows release ==="

if ! command -v x86_64-w64-mingw32-gcc &>/dev/null; then
    echo "WARNING: mingw-w64 not found. Install: sudo apt install mingw-w64"
    echo "Skipping Windows build."
else
    rustup target add x86_64-pc-windows-gnu 2>/dev/null || true
    cargo build --release --target x86_64-pc-windows-gnu
    x86_64-w64-mingw32-strip target/x86_64-pc-windows-gnu/release/mediaforge.exe 2>/dev/null || true

    mkdir -p dist/mediaforge-windows
    cp target/x86_64-pc-windows-gnu/release/mediaforge.exe dist/mediaforge-windows/
    mkdir -p dist/mediaforge-windows/ffmpeg
    cp vendor/ffmpeg/ffmpeg.exe dist/mediaforge-windows/ffmpeg/
    cat > dist/mediaforge-windows/README.txt << 'EOF'
MediaForge - Portable Media Converter
======================================

Run:
  mediaforge.exe

ffmpeg is bundled — no installation needed.
Just extract and run.
EOF

    cd dist && zip -r "mediaforge-v${VERSION}-windows-x86_64.zip" mediaforge-windows && cd ..
    echo "  -> dist/mediaforge-v${VERSION}-windows-x86_64.zip"
    ls -lh "dist/mediaforge-v${VERSION}-windows-x86_64.zip"
fi

echo ""
echo "=== Release complete ==="
echo ""
ls -lh dist/*.tar.gz dist/*.zip 2>/dev/null || true
echo ""
echo "Files ready for distribution in dist/"
