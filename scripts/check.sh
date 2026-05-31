#!/usr/bin/env bash
# Cratua Media Forge — Full test suite
set -euo pipefail

echo "=== Rust Tests (core) ==="
cargo test -p mediaforge-core

echo ""
echo "=== Rust Tests (tauri) ==="
cargo test -p mediaforge-tauri --lib

echo ""
echo "=== JS Tests ==="
node --test crates/mediaforge-tauri/ui/test.js

echo ""
echo "=== Cargo Check ==="
cargo check -p mediaforge-core 2>&1

echo ""
echo "=== All checks passed ==="
