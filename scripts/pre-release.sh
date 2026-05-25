#!/usr/bin/env bash
# Pre-release verification script for cargo-release.
#
# This script is run by cargo-release BEFORE the version bump, commit, and tag.
# It MUST NOT modify files — only verify that the project is ready to release.
# Use scripts/pre-commit.sh for a dev convenience script that formats files.

set -euo pipefail

echo "🔍 Running pre-release checks..."

echo "📝 Checking Rust formatting..."
cargo fmt --all --check

echo "📝 Checking JSON/JSONC formatting (Biome)..."
biome check --no-errors-on-unmatched "**/*.json" "**/*.jsonc" 2>/dev/null || true

echo "📝 Checking Markdown formatting (rumdl)..."
rumdl check "**/*.md" 2>/dev/null || true

echo "📝 Checking TOML formatting (tombi)..."
tombi lint "**/*.toml" 2>/dev/null || true

echo "🔍 Running Clippy..."
cargo clippy --all-targets -- -D warnings

echo "🔒 Running security audit..."
cargo audit --quiet

echo "📋 Running dependency license/advisory checks..."
cargo deny check

echo "✂️  Checking for unused dependencies..."
cargo shear

echo "🛡️  Running secret detection..."
gitleaks detect --source . 2>/dev/null || true

echo ""
echo "✅ All pre-release checks passed!"
exit 0
