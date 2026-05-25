#!/usr/bin/env bash
# Dev convenience script: formats files and runs all quality checks.
# For a check-only script (used by cargo-release), use scripts/pre-release.sh.

set -euo pipefail

echo "🔧 Running pre-commit checks..."

echo "📝 Formatting Rust files..."
cargo fmt --all

echo "📝 Formatting JSON/JSONC files (Biome)..."
biome check --write --no-errors-on-unmatched "**/*.json" "**/*.jsonc" 2>/dev/null || true

echo "📝 Formatting JS/TS/CSS/HTML/GraphQL/YAML files (oxfmt)..."
aube x oxfmt "**/*.js" "**/*.jsx" "**/*.ts" "**/*.tsx" "**/*.mjs" "**/*.mts" "**/*.cjs" "**/*.cts" "**/*.css" "**/*.html" "**/*.graphql" "**/*.gql" "**/*.yaml" "**/*.yml" 2>/dev/null || true

echo "📝 Formatting Markdown files (rumdl)..."
rumdl fmt "**/*.md" 2>/dev/null || true
rumdl check "**/*.md" 2>/dev/null || true

echo "📝 Formatting TOML files (tombi)..."
tombi format "**/*.toml" 2>/dev/null || true
tombi lint "**/*.toml" 2>/dev/null || true

echo "🔍 Running Clippy..."
cargo clippy --all-targets -- -D warnings

echo "🔒 Running security audit..."
cargo audit --quiet

echo "📋 Running dependency license/advisory checks..."
cargo deny check

echo "✂️  Checking for unused dependencies..."
cargo shear

echo "🛡️  Running secret detection on staged changes..."
gitleaks protect --staged

echo ""
echo "✅ All pre-commit checks passed!"
exit 0
