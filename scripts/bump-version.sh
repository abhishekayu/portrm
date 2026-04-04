#!/bin/bash
# Bump version across all package files and create a git tag.
# Usage: ./scripts/bump-version.sh 0.2.0
set -euo pipefail

if [ $# -ne 1 ]; then
    echo "Usage: $0 <new-version>" >&2
    echo "Example: $0 0.2.0" >&2
    exit 1
fi

NEW_VERSION="$1"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"

echo "Bumping to v${NEW_VERSION}..."

# Cargo.toml
sed -i '' "s/^version = \".*\"/version = \"${NEW_VERSION}\"/" "$ROOT/Cargo.toml"
echo "  Updated Cargo.toml"

# npm/package.json
sed -i '' "s/\"version\": \".*\"/\"version\": \"${NEW_VERSION}\"/" "$ROOT/npm/package.json"
echo "  Updated npm/package.json"

# Formula/portrm.rb
sed -i '' "s/version \".*\"/version \"${NEW_VERSION}\"/" "$ROOT/Formula/portrm.rb"
echo "  Updated Formula/portrm.rb"

# Update Cargo.lock
cd "$ROOT" && cargo check --quiet 2>/dev/null || true
echo "  Updated Cargo.lock"

echo ""
echo "Version bumped to v${NEW_VERSION}"
echo ""
echo "Next steps:"
echo "  git add -A"
echo "  git commit -m 'release: v${NEW_VERSION}'"
echo "  git tag v${NEW_VERSION}"
echo "  git push origin main --tags"
