# Zed Knip

This Zed extension wraps [Knip](https://knip.dev/) - the linter that spots unused files, dependencies, and exports in JS/TS projects.

## What it does

Zed Knip runs the Knip language server in the background, flagging unused code and dependencies as you work.

## Features

- **Diagnostics**: See warnings for unused files, dependencies, exports, and circular dependencies right in your editor.
- **Code Actions**: Quick fixes to ditch unused exports or dependencies, plus a way to mark exports as intentional with JSDoc tags.
- **Hover Support**: Hover over exports or dependencies to see how many times they're used.
- **Managed Installation**: The extension grabs and manages the Knip language server for you if it's missing from your workspace.
- **Package Manager Detection**: Works with npm, pnpm, yarn, or bun without any extra config.

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

Configure the extension under `lsp.knip` in your Zed `settings.json`. Settings here override anything auto-detected from your workspace.

### `lsp.knip.settings`

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
| `server_path` | `string?` | `null` | Explicit path to the Knip language server binary. |
| `package_manager` | `string?` | `null` | Override detected package manager: `npm`, `pnpm`, `yarn`, `bun`. |
| `auto_install` | `boolean` | `true` | Let the extension download and manage Knip when it's not found locally. |
| `log_level` | `string` | `"info"` | Server log verbosity: `trace`, `debug`, `info`, `warn`, `error`. |
| `config_path` | `string?` | `null` | Explicit path to your Knip config file. |
| `require_config` | `boolean` | `false` | Only start the server if a Knip config file exists. |

### `lsp.knip.binary`

Zed's standard binary configuration for controlling how the language server process is launched:

```json
{
  "lsp": {
    "knip": {
      "binary": {
        "path": "/path/to/knip-language-server",
        "arguments": ["--no-gitignore"],
        "env": {
          "KNIP_LOG_LEVEL": "debug",
          "KNIP_PACKAGE_MANAGER": "pnpm"
        }
      }
    }
  }
}
```

| Field | Type | Description |
| :--- | :--- | :--- |
| `path` | `string?` | Explicit path to the Knip language server binary. Same as `settings.server_path`. |
| `arguments` | `string[]` | Extra CLI flags forwarded to the language server. |
| `env.KNIP_LOG_LEVEL` | `string` | Server log verbosity. Overrides `settings.log_level`. |
| `env.KNIP_PACKAGE_MANAGER` | `string` | Package manager override. Overrides `settings.package_manager`. |

### Precedence

Override order, highest to lowest:

1. JSON — `lsp.knip.settings.*` — individual field overrides
2. Binary — `lsp.knip.binary.*` — `path`, `arguments`, and `env` variables
3. Defaults — built-in values

If both `lsp.knip.binary.arguments` and `lsp.knip.settings` are configured for extra args, binary wins.

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
