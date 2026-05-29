#!/usr/bin/env bash
# Bump version across all project files using semantic versioning.
# Usage: ./scripts/bump.sh patch|minor|major
set -euo pipefail

LEVEL="${1:-}"
if [ "$LEVEL" != "patch" ] && [ "$LEVEL" != "minor" ] && [ "$LEVEL" != "major" ]; then
  echo "Usage: $0 patch|minor|major"
  echo "  patch = x.y.Z+1  (bug fixes)"
  echo "  minor = x.Y+1.0  (new features, backwards compatible)"
  echo "  major = X+1.0.0  (breaking changes)"
  exit 1
fi

ROOT="$(cd "$(dirname "$0")/.." && pwd)"

# ── Read current version from Cargo.toml ──
CURRENT=$(grep -oP '^version\s*=\s*"\K[^"]+' "$ROOT/Cargo.toml" | head -1)
if [ -z "$CURRENT" ]; then
  echo "ERROR: Could not find version in Cargo.toml"
  exit 1
fi

echo "Current version: $CURRENT"

# ── Bump semver ──
IFS='.' read -r MAJOR MINOR PATCH <<< "$CURRENT"
case "$LEVEL" in
  major) MAJOR=$((MAJOR + 1)); MINOR=0; PATCH=0 ;;
  minor) MINOR=$((MINOR + 1)); PATCH=0 ;;
  patch) PATCH=$((PATCH + 1)) ;;
esac
NEW="$MAJOR.$MINOR.$PATCH"

echo "New version:     $NEW"
echo ""

# ── Update files ──
echo "Updating Cargo.toml..."
sed -i "s/^version\s*=\s*\"$CURRENT\"/version = \"$NEW\"/" "$ROOT/Cargo.toml"

echo "Updating tauri.conf.json..."
sed -i "s/\"version\": \"$CURRENT\"/\"version\": \"$NEW\"/" "$ROOT/crates/mediaforge-tauri/tauri.conf.json"

echo "Updating package.json..."
sed -i "s/\"version\": \"$CURRENT\"/\"version\": \"$NEW\"/" "$ROOT/crates/mediaforge-tauri/package.json"

# ── Verify ──
echo ""
echo "Verifying..."
grep "version" "$ROOT/Cargo.toml" | head -1
grep '"version"' "$ROOT/crates/mediaforge-tauri/tauri.conf.json"
grep '"version"' "$ROOT/crates/mediaforge-tauri/package.json"

# ── Git tag ──
echo ""
read -rp "Create git tag v$NEW? [Y/n] " REPLY
if [ "${REPLY:-y}" = "y" ] || [ "${REPLY:-y}" = "Y" ]; then
  git add "$ROOT/Cargo.toml" "$ROOT/crates/mediaforge-tauri/tauri.conf.json" "$ROOT/crates/mediaforge-tauri/package.json"
  git commit -m "chore: bump version to $NEW"
  git tag "v$NEW"
  echo "Tag v$NEW created. Use 'git push --tags' to push."
fi

echo ""
echo "Done. Version bumped: $CURRENT → $NEW"
