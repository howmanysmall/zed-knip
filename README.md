# Zed Knip

Zed extension for [Knip](https://knip.dev/), the project linter for unused files, dependencies, and exports in JavaScript and TypeScript projects.

## Overview

Zed Knip integrates the Knip language server into Zed, providing real-time feedback on unused code and dependencies directly in your editor.

## Features

- **Diagnostics**: Real-time warnings for unused files, unused dependencies, unused exports, and circular dependencies.
- **Code Actions**: Quick fixes to remove unused exports or dependencies, and the ability to mark exports as intentional via JSDoc tags.
- **Hover Support**: View usage counts for exports and dependencies by hovering over them.
- **Managed Installation**: Automatically downloads and manages the Knip language server if it's not found in your workspace.
- **Package Manager Detection**: Supports npm, pnpm, yarn, and bun out of the box.
- **Slash Commands**:
    - `/knip-report`: Generates a summary of all issues in the workspace.
    - `/knip-imports`: Lists unused import counts.
    - `/knip-exports`: Lists unused export counts.
    - `/knip-start`: Manually starts the Knip session.
    - `/knip-restart`: Restarts the Knip session.

## Installation

### Local Development Install

1. Build the extension WASM artifact:

   ```sh
   mise x -- cargo build --release --target wasm32-wasip2
   ```

2. Copy the extension directory to your local Zed extensions folder:

   - **macOS**: `~/Library/Application Support/Zed/extensions/installed/zed-knip/`
   - **Linux**: `~/.local/share/zed/extensions/installed/zed-knip/`

   The directory should contain `extension.toml` and the compiled `extension.wasm` (rename `target/wasm32-wasip2/release/zed_knip.wasm` to `extension.wasm`).

3. Restart Zed to load the extension.

## Settings

Configure the extension in your Zed `settings.json`:

```json
{
  "lsp": {
    "knip": {
      "settings": {
        "auto_install": true,
        "log_level": "info",
        "require_config": false
      }
    }
  }
}
```

| Setting | Type | Default | Description |
| :--- | :--- | :--- | :--- |
| `server_path` | `string` | `null` | Explicit path to the Knip language server binary. |
| `package_manager` | `string` | `null` | Override for the detected package manager (npm, pnpm, yarn, bun). |
| `auto_install` | `boolean` | `true` | Enable managed installation of Knip. |
| `log_level` | `string` | `"info"` | Log verbosity for the server (trace, debug, info, warn, error). |
| `extra_args` | `string[]` | `[]` | Extra CLI arguments forwarded to the language server. |
| `config_path` | `string` | `null` | Explicit path to your Knip configuration file. |
| `require_config` | `boolean` | `false` | If true, the server only starts if a config file is found. |

## Development

### Build and Test

```sh
# Run tests
mise x -- cargo nextest run --no-tests=pass

# Run all targets and features
mise x -- cargo nextest run --all-targets --all-features

# Build for WASM
mise x -- cargo build --release --target wasm32-wasip2
```

### Formatting and Linting

```sh
# Format files
npm run format

# Run clippy
cargo clippy --all-targets -- -D warnings
```

## Architecture

| Module | Purpose |
| :--- | :--- |
| `lib.rs` | Extension entry point — registers `ZedKnipExtension`. |
| `config_detection.rs` | Detects Knip configuration files in workspaces. |
| `package_manager.rs` | Detects the workspace package manager via lockfiles. |
| `resolver.rs` | Resolves the Knip executable path. |
| `settings.rs` | Extension settings schema. |
| `cache.rs` | Managed install caching. |
| `managed_install.rs` | Downloads and manages Knip binary installs. |
| `reports.rs` | Parses and formats Knip output reports. |
| `logging.rs` | Extension logging utilities. |
| `errors.rs` | Central error handling for the extension. |

## License

MIT
