#!/usr/bin/env bash
# Release: download ffmpeg → build → sign → bundle for Linux and Windows
# Usage: bash scripts/release.sh [version]
set -euo pipefail

cd "$(dirname "$0")/.."

VERSION="${1:-0.1.0}"
echo ""
echo "============================================"
echo "  Cratua Media Forge — Release v${VERSION}"
echo "============================================"

# ── Pre-flight: ffmpeg ──
if [ ! -f "vendor/ffmpeg/ffmpeg" ] || [ ! -f "vendor/ffmpeg/ffmpeg.exe" ]; then
    echo ""
    echo ">>> Downloading ffmpeg..."
    bash scripts/download-ffmpeg.sh
fi

# ── Pre-flight: code signing cert ──
if [ ! -f "certs/cratua.pfx" ]; then
    echo ""
    echo ">>> Generating self-signed code signing certificate..."
    bash scripts/generate-cert.sh
fi

rm -rf dist
mkdir -p dist

# ═══════════════════════════════════════════
# ── Linux ──
# ═══════════════════════════════════════════
echo ""
echo "=== [1/3] Building Linux release ==="
cargo build --release
strip target/release/mediaforge 2>/dev/null || true

mkdir -p dist/mediaforge-linux/ffmpeg
cp target/release/mediaforge dist/mediaforge-linux/
cp vendor/ffmpeg/ffmpeg dist/mediaforge-linux/ffmpeg/
cat > dist/mediaforge-linux/README.txt << 'EOF'
Cratua Media Forge — Portable Media Converter
===============================================

Run:
  ./mediaforge

For HiDPI monitors:
  WINIT_X11_SCALE_FACTOR=2 ./mediaforge

ffmpeg is bundled — no installation needed.
EOF

cd dist && tar czf "cratua-media-forge-v${VERSION}-linux-x86_64.tar.gz" mediaforge-linux && cd ..
echo "  -> dist/cratua-media-forge-v${VERSION}-linux-x86_64.tar.gz"
ls -lh "dist/cratua-media-forge-v${VERSION}-linux-x86_64.tar.gz"

# ═══════════════════════════════════════════
# ── Windows ──
# ═══════════════════════════════════════════
echo ""
echo "=== [2/3] Cross-compiling Windows release ==="

if ! command -v x86_64-w64-mingw32-gcc &>/dev/null; then
    echo "WARNING: mingw-w64 not found. Install: sudo apt install mingw-w64"
    echo "Skipping Windows build."
else
    rustup target add x86_64-pc-windows-gnu 2>/dev/null || true
    cargo build --release --target x86_64-pc-windows-gnu
    x86_64-w64-mingw32-strip target/x86_64-pc-windows-gnu/release/mediaforge.exe 2>/dev/null || true

    mkdir -p dist/mediaforge-windows/ffmpeg
    cp target/x86_64-pc-windows-gnu/release/mediaforge.exe dist/mediaforge-windows/
    cp vendor/ffmpeg/ffmpeg.exe dist/mediaforge-windows/ffmpeg/
    cat > dist/mediaforge-windows/README.txt << 'EOF'
Cratua Media Forge — Portable Media Converter
===============================================

Run:
  mediaforge.exe

ffmpeg is bundled — no installation needed.
Just extract and run.
EOF

    # ── Sign Windows binary ──
    echo ""
    echo "=== [3/3] Signing Windows binary ==="
    if command -v osslsigncode &>/dev/null; then
        EXE="dist/mediaforge-windows/mediaforge.exe"
        osslsigncode sign \
            -pkcs12 certs/cratua.pfx \
            -pass "" \
            -n "Cratua Media Forge" \
            -i "https://cratua.com" \
            -t "http://timestamp.digicert.com" \
            -in "$EXE" \
            -out "${EXE}.signed" 2>/dev/null && \
            mv "${EXE}.signed" "$EXE" && \
            echo "  -> Signed: $EXE" || \
            echo "  -> Signing skipped (timestamp server unreachable, unsigned binary is fine)"
    else
        echo "  -> osslsigncode not found. Install: sudo apt install osslsigncode"
        echo "  -> Skipping signing. Binary is functional but unsigned."
    fi

    cd dist && zip -r "cratua-media-forge-v${VERSION}-windows-x86_64.zip" mediaforge-windows && cd ..
    echo "  -> dist/cratua-media-forge-v${VERSION}-windows-x86_64.zip"
    ls -lh "dist/cratua-media-forge-v${VERSION}-windows-x86_64.zip"
fi

# ═══════════════════════════════════════════
echo ""
echo "============================================"
echo "  Release v${VERSION} complete"
echo "============================================"
echo ""
ls -lh dist/*.tar.gz dist/*.zip 2>/dev/null || true
echo ""
echo "Artifacts in dist/"
