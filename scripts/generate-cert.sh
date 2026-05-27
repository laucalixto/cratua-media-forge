#!/usr/bin/env bash
# Generate a self-signed code signing certificate for Windows
# Outputs: certs/cratua.pfx (can be used with signtool or osslsigncode)
set -euo pipefail

cd "$(dirname "$0")/.."
mkdir -p certs

CERT_NAME="Cratua Media Forge"
CERT_FILE="certs/cratua"

echo "=== Generating self-signed code signing certificate ==="

# Generate private key
openssl genrsa -out "${CERT_FILE}.key" 2048

# Generate certificate (valid 5 years)
openssl req -new -x509 -days 1825 \
    -key "${CERT_FILE}.key" \
    -out "${CERT_FILE}.crt" \
    -subj "/C=BR/ST=Sao Paulo/L=Sao Paulo/O=Cratua/CN=Cratua Media Forge"

# Bundle into PFX (PKCS#12) — Windows uses this format
openssl pkcs12 -export \
    -in "${CERT_FILE}.crt" \
    -inkey "${CERT_FILE}.key" \
    -out "${CERT_FILE}.pfx" \
    -passout pass:""  # No password for CI convenience

echo ""
echo "=== Certificate generated ==="
echo "Certificate: ${CERT_FILE}.crt"
echo "Private key: ${CERT_FILE}.key"
echo "PFX bundle:  ${CERT_FILE}.pfx"
echo ""
echo "To sign the Windows .exe:"
echo "  bash scripts/sign-windows.sh"
echo ""
echo "IMPORTANT: This is a SELF-SIGNED certificate."
echo "Users will see 'Unknown Publisher' until you get a real code signing cert."
echo "For production, buy from DigiCert, Sectigo, or use Azure Trusted Signing."
