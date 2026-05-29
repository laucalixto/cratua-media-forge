#!/usr/bin/env bash
# Release: bump all version files and create a git tag.
# Usage: ./scripts/release.sh 0.2.0
set -euo pipefail

VERSION="${1:-}"
if [ -z "$VERSION" ]; then
  echo "Usage: $0 <version>"
  echo "Example: $0 0.2.0"
  exit 1
fi

# Validate semver format
if ! echo "$VERSION" | grep -qP '^\d+\.\d+\.\d+$'; then
  echo "ERROR: '$VERSION' is not a valid semver (x.y.z)"
  exit 1
fi

ROOT="$(cd "$(dirname "$0")/.." && pwd)"

# ── Read current version ──
CURRENT=$(grep -oP '^version\s*=\s*"\K[^"]+' "$ROOT/Cargo.toml" | head -1)
if [ -z "$CURRENT" ]; then
  echo "ERROR: Could not find version in Cargo.toml"
  exit 1
fi

if [ "$CURRENT" = "$VERSION" ]; then
  echo "Version is already $VERSION"
  echo ""
  read -rp "Build release artifacts anyway? [Y/n] " REPLY
  if [ "${REPLY:-y}" != "y" ] && [ "${REPLY:-y}" != "Y" ]; then
    exit 0
  fi
  # Jump directly to build (skip bump + commit)
  SKIP_BUMP=1
fi

if [ "${SKIP_BUMP:-0}" != "1" ]; then

echo "Bumping: $CURRENT → $VERSION"
echo ""

# ── Update files ──
sed -i "s/^version\s*=\s*\"$CURRENT\"/version = \"$VERSION\"/" "$ROOT/Cargo.toml"
sed -i "s/\"version\": \"$CURRENT\"/\"version\": \"$VERSION\"/" "$ROOT/crates/mediaforge-tauri/tauri.conf.json"
sed -i "s/\"version\": \"$CURRENT\"/\"version\": \"$VERSION\"/" "$ROOT/crates/mediaforge-tauri/package.json"

# ── Verify ──
echo "Updated files:"
grep "^version" "$ROOT/Cargo.toml" | head -1
grep '"version"' "$ROOT/crates/mediaforge-tauri/tauri.conf.json"
grep '"version"' "$ROOT/crates/mediaforge-tauri/package.json"

# ── Commit + tag ──
echo ""
read -rp "Commit and tag v$VERSION? [Y/n] " REPLY
if [ "${REPLY:-y}" != "y" ] && [ "${REPLY:-y}" != "Y" ]; then
  echo "Skipped. Files are updated but not committed."
  exit 0
fi

git add \
  "$ROOT/Cargo.toml" \
  "$ROOT/crates/mediaforge-tauri/tauri.conf.json" \
  "$ROOT/crates/mediaforge-tauri/package.json"
git commit -m "chore: release v$VERSION"
git tag "v$VERSION"

echo ""
echo "Released v$VERSION. Run 'git push --tags' to publish."

fi   # end SKIP_BUMP block

# ═══════════════════════════════════════════
# 1. Build frontend (shared)
# ═══════════════════════════════════════════
echo ""
echo "=== [1/4] Building frontend ==="
# ── Pre-flight: ensure ffmpeg/ffprobe are available ──
NEED_DL=0
[ -f "$ROOT/vendor/ffmpeg/ffmpeg" ] || NEED_DL=1
[ -f "$ROOT/vendor/ffmpeg/ffmpeg.exe" ] || NEED_DL=1
[ -f "$ROOT/vendor/ffmpeg/ffprobe.exe" ] || NEED_DL=1
if [ "$NEED_DL" = "1" ]; then
  echo ">>> Downloading ffmpeg + ffprobe..."
  bash "$ROOT/scripts/download-ffmpeg.sh"
fi

rm -rf "$ROOT/dist"
mkdir -p "$ROOT/dist"
cd "$ROOT/crates/mediaforge-tauri"
npm run build --silent 2>/dev/null
cd "$ROOT"

# ═══════════════════════════════════════════
# 2. Linux
# ═══════════════════════════════════════════
echo ""
echo "=== [2/4] Building Linux release ==="
rm -rf "$ROOT/target/release/bundle"
cd "$ROOT/crates/mediaforge-tauri"
cargo tauri build 2>&1 | grep -E "Finished|Bundling|Error|Built" || true
cd "$ROOT"

BIN_LINUX="$ROOT/target/release/mediaforge-tauri"
if [ -f "$BIN_LINUX" ]; then
  strip "$BIN_LINUX" 2>/dev/null || true
  mkdir -p "$ROOT/dist/cratua-media-forge-linux/ffmpeg"
  cp "$BIN_LINUX" "$ROOT/dist/cratua-media-forge-linux/cratua-media-forge"
  cp "$ROOT/vendor/ffmpeg/ffmpeg" "$ROOT/dist/cratua-media-forge-linux/ffmpeg/"
  cat > "$ROOT/dist/cratua-media-forge-linux/README.txt" << 'EOF'
Cratua Media Forge — Portable Media Converter
===============================================
Run: ./cratua-media-forge
HiDPI: GDK_DPI_SCALE=2 ./cratua-media-forge
ffmpeg is bundled — no installation needed.
EOF
  cd "$ROOT/dist" && tar czf "cratua-media-forge-v${VERSION}-linux-x86_64.tar.gz" cratua-media-forge-linux && cd "$ROOT"
  echo "  -> dist/cratua-media-forge-v${VERSION}-linux-x86_64.tar.gz"
  ls -lh "$ROOT/dist/cratua-media-forge-v${VERSION}-linux-x86_64.tar.gz"
else
  echo "ERROR: Linux binary not found at $BIN_LINUX"
fi

# Also copy .deb if generated
DEB=$(ls "$ROOT/target/release/bundle/deb/"*.deb 2>/dev/null | head -1 || true)
if [ -n "$DEB" ]; then
  cp "$DEB" "$ROOT/dist/"
  echo "  -> dist/$(basename "$DEB")"
fi

# ═══════════════════════════════════════════
# 3. Windows (cross-compile)
# ═══════════════════════════════════════════
echo ""
echo "=== [3/4] Cross-compiling Windows release ==="

if ! command -v x86_64-w64-mingw32-gcc &>/dev/null; then
  echo "WARNING: mingw-w64 not found. Skipping Windows build."
  echo "  Install: sudo apt install mingw-w64"
else
  rustup target add x86_64-pc-windows-gnu 2>/dev/null || true
  cd "$ROOT/crates/mediaforge-tauri"
  cargo tauri build --target x86_64-pc-windows-gnu 2>&1 | grep -E "Finished|Bundling|Error|Built|Info" || true
  cd "$ROOT"

  BIN_WIN="$ROOT/target/x86_64-pc-windows-gnu/release/mediaforge-tauri.exe"
  if [ -f "$BIN_WIN" ]; then
    x86_64-w64-mingw32-strip "$BIN_WIN" 2>/dev/null || true
    mkdir -p "$ROOT/dist/cratua-media-forge-windows/ffmpeg"
    cp "$BIN_WIN" "$ROOT/dist/cratua-media-forge-windows/cratua-media-forge.exe"
    cp "$ROOT/vendor/ffmpeg/ffmpeg.exe" "$ROOT/dist/cratua-media-forge-windows/ffmpeg/"
    # ffprobe.exe needed for progress tracking
    [ -f "$ROOT/vendor/ffmpeg/ffprobe.exe" ] && cp "$ROOT/vendor/ffmpeg/ffprobe.exe" "$ROOT/dist/cratua-media-forge-windows/ffmpeg/"

    WEBVIEW2_DLL="$ROOT/target/x86_64-pc-windows-gnu/release/WebView2Loader.dll"
    if [ -f "$WEBVIEW2_DLL" ]; then
      cp "$WEBVIEW2_DLL" "$ROOT/dist/cratua-media-forge-windows/"
    fi

    cat > "$ROOT/dist/cratua-media-forge-windows/README.txt" << 'EOF'
Cratua Media Forge — Portable Media Converter
===============================================
Run: cratua-media-forge.exe
Requires Windows 10+ or WebView2 runtime.
ffmpeg is bundled — no installation needed.
EOF
    cd "$ROOT/dist" && zip -r "cratua-media-forge-v${VERSION}-windows-x86_64.zip" cratua-media-forge-windows && cd "$ROOT"
    echo "  -> dist/cratua-media-forge-v${VERSION}-windows-x86_64.zip"
    ls -lh "$ROOT/dist/cratua-media-forge-v${VERSION}-windows-x86_64.zip"
  else
    echo "WARNING: Windows binary not found"
  fi
fi

# ═══════════════════════════════════════════
# 4. Sign Windows binary (optional)
# ═══════════════════════════════════════════
echo ""
echo "=== [4/4] Signing Windows binary ==="
if command -v osslsigncode &>/dev/null && [ -f "$ROOT/certs/cratua-cert-dev.pfx" ]; then
  EXE="$ROOT/dist/cratua-media-forge-windows/cratua-media-forge.exe"
  if [ -f "$EXE" ]; then
    osslsigncode sign \
      -h sha256 \
      -pkcs12 "$ROOT/certs/cratua-cert-dev.pfx" -pass '9~>ByWH]Q.(H.ZNcn' \
      -n "Cratua Media Forge" \
      -i "https://cratua.com" \
      -t "http://timestamp.digicert.com" \
      -in "$EXE" -out "${EXE}.signed" && \
      mv "${EXE}.signed" "$EXE" && \
      echo "  -> Signed successfully" || \
      echo "  -> Signing FAILED — check cert password and network"
  fi
else
  if ! command -v osslsigncode &>/dev/null; then
    echo "  -> osslsigncode not installed. Install with: sudo apt install osslsigncode"
  elif [ ! -f "$ROOT/certs/cratua-cert-dev.pfx" ]; then
    echo "  -> cert not found at certs/cratua-cert-dev.pfx"
  else
    echo "  -> Signing skipped."
  fi
fi

# ═══════════════════════════════════════════
echo ""
echo "============================================"
echo "  Release v${VERSION} complete"
echo "============================================"
echo ""
ls -lh "$ROOT/dist/"*.tar.gz "$ROOT/dist/"*.zip "$ROOT/dist/"*.deb 2>/dev/null || true
echo ""
echo "Artifacts in dist/"