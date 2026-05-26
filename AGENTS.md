# Agent Guidelines

This file provides guidance to LLMs when working with code in this repository.

## Project Overview

**zed-knip** is a Zed editor extension that integrates Knip (JavaScript/TypeScript unused code detection) into the Zed
editor. It is a Rust-based Zed extension built as a `cdylib`.

## Commands

### Build & Test

```sh
# Run tests
mise x -- cargo nextest run --no-tests=pass

# Run a single test
mise x -- cargo nextest run <test_name>

# Format all files
npm run format

# Lint all files
aube run lint

# Type-check TypeScript
aube run type-check

# Full pre-commit checks (format + clippy + audit + deny + shear + gitleaks)
./scripts/pre-commit.sh
```

### Rust Quality (Scoped to Rust Files)

```sh
cargo fmt --all           # Format
cargo clippy --all-targets -- -D warnings  # Lint
cargo audit --quiet       # Security audit
cargo deny check          # License/advisory checks
cargo shear               # Unused dependencies
```

## Architecture

### Module Structure (`src/`)

| Module | Purpose |
|---|---|
| `lib.rs` | Extension entry point — registers `ZedKnipExtension` with the Zed extension API |
| `config_detection.rs` | Detects Knip configuration files in workspaces |
| `package_manager.rs` | Detects the workspace package manager (npm, pnpm, yarn, bun) via lockfiles and `packageManager` field |
| `resolver.rs` | Resolves the Knip executable path |
| `settings.rs` | Extension settings schema |
| `cache.rs` | Managed install caching |
| `managed_install.rs` | Downloads/manages Knip binary installs |
| `reports.rs` | Parses and formats Knip output reports |
| `logging.rs` | Extension logging utilities |
| `errors.rs` | `KnipError` enum with user-facing display messages |

Most modules are currently empty scaffolds — only `lib.rs`, `errors.rs`, and `package_manager.rs` have substantive implementations.

### Test Fixtures (`tests/fixtures/`)

Integration test fixtures simulate various workspace configurations: `npm`, `pnpm`, `yarn`, `bun`, `monorepo`,
`missing-knip`, `invalid-config`, `missing-config`, `multiple-lockfiles`, `path with spaces`. The `package_manager.rs`
tests use these to verify detection logic.

## Key Tooling

- **aube** — project package manager (replaces npm/pnpm/yarn in commands via `aube run`)
- **mise** — tool runner (`mise x -- <command>` to invoke dev tools)
- **lefthook** — git hooks (commit-msg, pre-commit, pre-push)
- **oxlint** + **biome** — linting
- **oxfmt** + **biome** — formatting
- **rumdl** — markdown formatting
- **tombi** — TOML formatting
- **gitleaks** — secret detection

## Conventions

- Git hooks enforce formatting, clippy, audit, deny, shear, and gitleaks on pre-commit
- Pre-push runs `cargo nextest`
- The `MODULE_COUNT` constant in `lib.rs` (currently 9) must be updated when adding/removing modules; the scaffold smoke
  test asserts it matches
- Use `rg` instead of `grep`, `fd` instead of `find`
