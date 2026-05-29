#!/usr/bin/env bash
# Download static ffmpeg builds for Linux and Windows
# Run from the project root
set -euo pipefail

cd "$(dirname "$0")/.."

FFMPEG_DIR="vendor/ffmpeg"
mkdir -p "$FFMPEG_DIR"

echo "=== Downloading static ffmpeg builds ==="
echo ""

# ── Linux (x86_64 static from johnvansickle.com) ──
echo "[1/2] Linux static build..."
LINUX_URL="https://johnvansickle.com/ffmpeg/releases/ffmpeg-release-amd64-static.tar.xz"
LINUX_TAR="$FFMPEG_DIR/ffmpeg-linux.tar.xz"

if [ ! -f "$FFMPEG_DIR/ffmpeg" ]; then
    curl -L -o "$LINUX_TAR" "$LINUX_URL"
    tar xf "$LINUX_TAR" -C "$FFMPEG_DIR" --strip-components=1
    rm "$LINUX_TAR"
    echo "  -> $FFMPEG_DIR/ffmpeg"
    ls -lh "$FFMPEG_DIR/ffmpeg"
else
    echo "  -> already exists, skipping"
fi

# ── Windows (x86_64 from gyan.dev) ──
echo ""
echo "[2/2] Windows static build..."
WIN_URL="https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.zip"
WIN_ZIP="$FFMPEG_DIR/ffmpeg-windows.zip"

if [ ! -f "$FFMPEG_DIR/ffmpeg.exe" ] || [ ! -f "$FFMPEG_DIR/ffprobe.exe" ]; then
    curl -L -o "$WIN_ZIP" "$WIN_URL"
    unzip -o "$WIN_ZIP" -d "$FFMPEG_DIR/win-tmp"
    # Find ffmpeg.exe in the extracted directory
    WIN_EXE=$(find "$FFMPEG_DIR/win-tmp" -name "ffmpeg.exe" -type f | head -1)
    if [ -n "$WIN_EXE" ]; then
        cp "$WIN_EXE" "$FFMPEG_DIR/ffmpeg.exe"
        WIN_PROBE=$(find "$FFMPEG_DIR/win-tmp" -name "ffprobe.exe" -type f | head -1)
        [ -n "$WIN_PROBE" ] && cp "$WIN_PROBE" "$FFMPEG_DIR/ffprobe.exe"
        echo "  -> $FFMPEG_DIR/ffmpeg.exe"
        ls -lh "$FFMPEG_DIR/ffmpeg.exe"
        [ -f "$FFMPEG_DIR/ffprobe.exe" ] && echo "  -> $FFMPEG_DIR/ffprobe.exe" && ls -lh "$FFMPEG_DIR/ffprobe.exe"
    fi
    rm -rf "$FFMPEG_DIR/win-tmp" "$WIN_ZIP"
else
    echo "  -> already exists, skipping"
fi

echo ""
echo "=== Done ==="
echo "Linux:   $FFMPEG_DIR/ffmpeg"
echo "Windows: $FFMPEG_DIR/ffmpeg.exe"
echo ""
echo "To bundle with the app, copy ffmpeg next to the mediaforge binary:"
echo "  cp $FFMPEG_DIR/ffmpeg dist/mediaforge-linux/"
echo "  cp $FFMPEG_DIR/ffmpeg.exe dist/mediaforge-windows/"