#!/usr/bin/env bash
# Sign the Windows .exe with the self-signed certificate
# Requires: osslsigncode (sudo apt install osslsigncode)
set -euo pipefail

cd "$(dirname "$0")/.."

EXE="dist/mediaforge-windows/mediaforge.exe"
CERT="certs/cratua.pfx"

if [ ! -f "$CERT" ]; then
    echo "Certificate not found. Generate it first:"
    echo "  bash scripts/generate-cert.sh"
    exit 1
fi

if ! command -v osslsigncode &>/dev/null; then
    echo "osslsigncode not found. Install: sudo apt install osslsigncode"
    exit 1
fi

if [ ! -f "$EXE" ]; then
    echo "$EXE not found. Build first: bash scripts/build-windows.sh"
    exit 1
fi

echo "=== Signing $EXE ==="

# Sign with timestamp (time stamping won't work without internet)
osslsigncode sign \
    -pkcs12 "$CERT" \
    -pass "" \
    -n "Cratua Media Forge" \
    -i "https://cratua.com" \
    -t "http://timestamp.digicert.com" \
    -in "$EXE" \
    -out "${EXE}.signed"

mv "${EXE}.signed" "$EXE"

echo "=== Signed: $EXE ==="
echo ""
echo "Verify with: osslsigncode verify $EXE"
echo "Or on Windows: signtool verify /pa $EXE"
