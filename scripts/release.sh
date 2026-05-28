#!/usr/bin/env bash
# Release: build frontend → compile → bundle → sign for Linux and Windows
# Usage: bash scripts/release.sh [version]
set -euo pipefail

cd "$(dirname "$0")/.."

VERSION="${1:-0.1.0}"
echo ""
echo "============================================"
echo "  Cratua Media Forge — Release v${VERSION}"
echo "============================================"

# ── Pre-flight ──
if [ ! -f "vendor/ffmpeg/ffmpeg" ] || [ ! -f "vendor/ffmpeg/ffmpeg.exe" ]; then
    echo ">>> Downloading ffmpeg..."
    bash scripts/download-ffmpeg.sh
fi

if [ ! -f "certs/cratua.pfx" ]; then
    echo ">>> Generating code signing certificate..."
    bash scripts/generate-cert.sh
fi

# Ensure icons exist
if [ ! -f "crates/mediaforge-tauri/icons/icon.ico" ]; then
    echo ">>> Generating icon.ico from icon.png..."
    python3 -c "
import struct
with open('crates/mediaforge-tauri/icons/icon.png','rb') as f: png = f.read()
header = struct.pack('<HHH', 0, 1, 1)
entry = struct.pack('<BBBBHHII', 0, 0, 0, 0, 1, 32, len(png), 22)
with open('crates/mediaforge-tauri/icons/icon.ico','wb') as f:
    f.write(header); f.write(entry); f.write(png)
print('icon.ico created')
"
fi

rm -rf dist
mkdir -p dist

# ═══════════════════════════════════════════
# 1. Build frontend (shared)
# ═══════════════════════════════════════════
echo ""
echo "=== [1/4] Building frontend ==="
cd crates/mediaforge-tauri
npm run build --silent 2>/dev/null
cd ../..

# ═══════════════════════════════════════════
# 2. Linux (Tauri build → .deb + portable .tar.gz)
# ═══════════════════════════════════════════
echo ""
echo "=== [2/4] Building Linux release ==="
cd crates/mediaforge-tauri
cargo tauri build 2>&1 | grep -E "Finished|Bundling|Error|Built"
cd ../..

BIN_LINUX="target/release/mediaforge-tauri"
if [ -f "$BIN_LINUX" ]; then
    strip "$BIN_LINUX" 2>/dev/null || true
    mkdir -p dist/cratua-media-forge-linux/ffmpeg
    cp "$BIN_LINUX" dist/cratua-media-forge-linux/cratua-media-forge
    cp vendor/ffmpeg/ffmpeg dist/cratua-media-forge-linux/ffmpeg/
    cat > dist/cratua-media-forge-linux/README.txt << 'EOF'
Cratua Media Forge — Portable Media Converter
===============================================
Run: ./cratua-media-forge
HiDPI: GDK_DPI_SCALE=2 ./cratua-media-forge
ffmpeg is bundled — no installation needed.
EOF
    cd dist && tar czf "cratua-media-forge-v${VERSION}-linux-x86_64.tar.gz" cratua-media-forge-linux && cd ..
    echo "  -> dist/cratua-media-forge-v${VERSION}-linux-x86_64.tar.gz"
    ls -lh "dist/cratua-media-forge-v${VERSION}-linux-x86_64.tar.gz"
fi

# Also copy .deb if generated
DEB=$(ls target/release/bundle/deb/*.deb 2>/dev/null | head -1 || true)
if [ -n "$DEB" ]; then
    cp "$DEB" "dist/"
    echo "  -> dist/$(basename "$DEB")"
fi

# ═══════════════════════════════════════════
# 3. Windows (cross-compile via Tauri)
# ═══════════════════════════════════════════
echo ""
echo "=== [3/4] Cross-compiling Windows release ==="

if ! command -v x86_64-w64-mingw32-gcc &>/dev/null; then
    echo "WARNING: mingw-w64 not found. Skipping Windows build."
else
    rustup target add x86_64-pc-windows-gnu 2>/dev/null || true
    cd crates/mediaforge-tauri
    cargo tauri build --target x86_64-pc-windows-gnu 2>&1 | grep -E "Finished|Bundling|Error|Built|Info"
    cd ../..

    BIN_WIN="target/x86_64-pc-windows-gnu/release/mediaforge-tauri.exe"
    if [ -f "$BIN_WIN" ]; then
        x86_64-w64-mingw32-strip "$BIN_WIN" 2>/dev/null || true
        mkdir -p dist/cratua-media-forge-windows/ffmpeg
        cp "$BIN_WIN" dist/cratua-media-forge-windows/cratua-media-forge.exe
        cp vendor/ffmpeg/ffmpeg.exe dist/cratua-media-forge-windows/ffmpeg/

        # Bundle WebView2Loader if present (MUST be next to .exe)
        WEBVIEW2_DLL="target/x86_64-pc-windows-gnu/release/WebView2Loader.dll"
        if [ -f "$WEBVIEW2_DLL" ]; then
            cp "$WEBVIEW2_DLL" dist/cratua-media-forge-windows/
        fi

        cat > dist/cratua-media-forge-windows/README.txt << 'EOF'
Cratua Media Forge — Portable Media Converter
===============================================
Run: cratua-media-forge.exe
Requires Windows 10+ or WebView2 runtime.
ffmpeg is bundled — no installation needed.
EOF
        cd dist && zip -r "cratua-media-forge-v${VERSION}-windows-x86_64.zip" cratua-media-forge-windows && cd ..
        echo "  -> dist/cratua-media-forge-v${VERSION}-windows-x86_64.zip"
        ls -lh "dist/cratua-media-forge-v${VERSION}-windows-x86_64.zip"
    fi
fi

# ═══════════════════════════════════════════
# 4. Sign Windows binary
# ═══════════════════════════════════════════
echo ""
echo "=== [4/4] Signing Windows binary ==="
if command -v osslsigncode &>/dev/null && [ -f "certs/cratua.pfx" ]; then
    EXE="dist/cratua-media-forge-windows/cratua-media-forge.exe"
    if [ -f "$EXE" ]; then
        osslsigncode sign \
            -pkcs12 certs/cratua.pfx -pass "" \
            -n "Cratua Media Forge" \
            -i "https://cratua.com" \
            -t "http://timestamp.digicert.com" \
            -in "$EXE" -out "${EXE}.signed" 2>/dev/null && \
            mv "${EXE}.signed" "$EXE" && \
            echo "  -> Signed successfully" || \
            echo "  -> Signing skipped (no network for timestamp)"
    fi
else
    echo "  -> osslsigncode not installed. Skipping."
fi

# ═══════════════════════════════════════════
echo ""
echo "============================================"
echo "  Release v${VERSION} complete"
echo "============================================"
echo ""
ls -lh dist/*.tar.gz dist/*.zip dist/*.deb 2>/dev/null || true
echo ""
echo "Artifacts in dist/"
