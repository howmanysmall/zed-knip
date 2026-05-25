#!/usr/bin/env bash
# Release script for zed-knip
#
# Usage:
#   ./scripts/release.sh patch
#   ./scripts/release.sh minor
#   ./scripts/release.sh major
#   ./scripts/release.sh 1.2.3
#
# This automates the full cargo-release flow with guardrails.

set -euo pipefail

# ── helpers ────────────────────────────────────────────────────────────

red() { printf '\033[0;31m%s\033[0m\n' "$1" >&2; }
green() { printf '\033[0;32m%s\033[0m\n' "$1"; }
yellow() { printf '\033[0;33m%s\033[0m\n' "$1"; }

die() {
    red "error: $1"
    exit 1
}

# ── pre-flight checks ──────────────────────────────────────────────────

# 1. Must be on main
branch=$(git branch --show-current 2> /dev/null || true)
if [ "$branch" != "main" ]; then
    die "Must be on 'main' branch (current: ${branch:-unknown}).  Switch with: git switch main"
fi

# 2. Working tree must be clean
if ! git diff-index --quiet HEAD -- 2> /dev/null; then
    die "Working tree is dirty.  Stash or commit changes before releasing."
fi

# 3. Version argument is required
if [ $# -lt 1 ]; then
    echo "Usage: $0 <major|minor|patch|VERSION>" >&2
    echo ""
    echo "Examples:"
    echo "  $0 patch          # bump patch version (1.0.0 → 1.0.1)"
    echo "  $0 minor          # bump minor version (1.0.0 → 1.1.0)"
    echo "  $0 major          # bump major version (1.0.0 → 2.0.0)"
    echo "  $0 1.2.3          # set exact version"
    exit 1
fi

version="$1"

# 4. cargo-release must be installed
if ! command -v cargo-release &> /dev/null && ! cargo release --version &> /dev/null 2>&1; then
    die "cargo-release is not installed.  Install with: cargo install cargo-release"
fi

# ── confirm ─────────────────────────────────────────────────────────────

current_ver=$(cargo metadata --format-version 1 --no-deps 2> /dev/null \
    | grep -m1 '"name":"zed-knip"' -A1 | tail -1 | sed -n 's/.*"version":"\([^"]*\)".*/\1/p')

yellow "Current version: $current_ver"
yellow "Bump:            $version"
echo ""
read -r -p "Proceed? [y/N] " answer
case "$answer" in
    [yY] | [yY][eE][sS]) ;;
    *) die "Aborted by user." ;;
esac

# ── release ─────────────────────────────────────────────────────────────

echo ""
green "→ Running cargo release $version …"
echo ""

# cargo-release handles:
#   - pre-release-hook (./scripts/pre-commit.sh)
#   - version bump in Cargo.toml
#   - build verification
#   - commit with message
#   - git tag
#   - push to origin
cargo release "$version" --execute

echo ""
green "✓ Release complete."
green "  The 'release.yml' CI workflow will now build and publish artifacts."
green "  Monitor progress at: https://github.com/howmanysmall/zed-knip/actions"
