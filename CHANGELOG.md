# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-05-25

### Added

- Initial release of the Knip extension for Zed.
- Support for detecting unused files, dependencies, and exports via LSP diagnostics.
- Managed installation of `knip-language-server` with automated updates and caching.
- Package manager detection for npm, pnpm, yarn, and bun.
- Workspace configuration detection for `knip.json`, `knip.jsonc`, and other supported formats.
- Slash commands for workspace reports and manual lifecycle control:
    - `/knip-report`: Generate a full workspace issue summary.
    - `/knip-imports`: View unused import counts.
    - `/knip-exports`: View unused export counts.
    - `/knip-start`: Manually start the language server.
    - `/knip-restart`: Restart the language server.
- Code actions for quick fixes:
    - Remove unused exports.
    - Add `@public` JSDoc tags to suppress warnings.
    - Remove unused dependencies from `package.json`.
- Hover information for export and dependency usage counts.
- Configurable settings for server paths, package manager overrides, and log levels.
