# Zed Knip

This Zed extension wraps [Knip](https://knip.dev/) — the linter that spots unused files, dependencies, and exports in JS/TS projects.

## What it does

Zed Knip runs the Knip language server in the background, flagging unused code and dependencies as you work.

## Features

- **Diagnostics**: See warnings for unused files, dependencies, exports, and circular dependencies right in your editor.
- **Code Actions**: Quick fixes to ditch unused exports or dependencies from upstream Knip (`CodeActionKind.QuickFix`).
- **Managed Installation**: The extension manages the `@knip/language-server` install for you if it is missing from your workspace.
- **Advanced Diagnostic Filtering**: Fine-grained control over which Knip issues are reported.
- **Custom TS Config**: Support for alternate TypeScript configurations via `ts_config_path`.
- **Optional Explicit Config**: Point the extension at a specific Knip configuration file.

## Settings

Configure the extension under `lsp.knip` in your Zed `settings.json`.

### `lsp.knip.settings`

```json
{
  "lsp": {
    "knip": {
      "settings": {
        "auto_install": true,
        "config_path": "knip.json",
        "require_config": false,
        "ts_config_path": "tsconfig.knip.json",
        "preprocessor": ["./tools/knip-preprocessor.mjs", "knip-preprocessor-package"],
        "preprocessor_options": { "key": "value" },
        "diagnostics": {
          "include_issue_types": ["files", "dependencies"],
          "exclude_issue_types": ["duplicates"],
          "exclude_path_prefixes": ["src/legacy/"],
          "severity_by_issue_type": {
            "unlisted": "error",
            "exports": "warn"
          }
        }
      }
    }
  }
}
```

| Setting | Type | Default | Description |
| :--- | :--- | :--- | :--- |
| `auto_install` | `boolean` | `true` | Let the extension download and manage Knip when it is not found locally. |
| `config_path` | `string?` | `null` | Explicit path to your Knip config file. |
| `require_config` | `boolean` | `false` | Only start the server if a Knip config file exists. |
| `ts_config_path` | `string?` | `null` | Alternate TS config path. **Requires Managed Install.** |
| `diagnostics.include_issue_types` | `array` | `[]` | Issue types to include. Defaults to all if empty. **Requires Managed Install.** |
| `diagnostics.exclude_issue_types` | `array` | `[]` | Issue types to exclude (wins over include). **Requires Managed Install.** |
| `diagnostics.exclude_path_prefixes` | `array` | `[]` | POSIX-normalized path prefixes to ignore. **Requires Managed Install.** |
| `diagnostics.severity_by_issue_type` | `object` | `{}` | Map of issue types to severity levels. **Requires Managed Install.** |
| `preprocessor` | `array` | `[]` | List of preprocessor scripts or packages to run. **Requires Managed Install.** |
| `preprocessor_options` | `object` | `{}` | Options passed to preprocessors. **Requires Managed Install.** |

### Trusted Code Warning

Preprocessors execute trusted workspace or package JavaScript. Only use preprocessors in repositories you trust.

### Managed Install Requirement

Preprocessors require a **Managed Install** of Knip. Using a custom `lsp.knip.binary.path` will cause preprocessor settings to be ignored.

### `lsp.knip.binary`

Zed standard binary configuration for custom language server paths:

```json
{
  "lsp": {
    "knip": {
      "binary": {
        "path": "/path/to/custom/knip-language-server"
      }
    }
  }
}
```

**Note:** Using `binary.path` for a baseline custom language server does NOT support advanced features like `ts_config_path` or `diagnostics` filters.

## Diagnostic Filtering Semantics

The `diagnostics` object allows you to filter which issues are reported by the language server.

- `include_issue_types`: Defaults to all types if empty.
- `exclude_issue_types`: Takes precedence over `include_issue_types`.
- `exclude_path_prefixes`: POSIX-normalized paths relative to workspace root.
- `severity_by_issue_type`: Map issue types to `error`, `warn`, `info`, `hint`, or `off`.

### Valid Issue Types

The following 15 issue types are supported:
`files`, `dependencies`, `devDependencies`, `optionalPeerDependencies`, `unlisted`, `binaries`, `unresolved`, `exports`, `types`, `nsExports`, `nsTypes`, `duplicates`, `enumMembers`, `namespaceMembers`, `catalog`.

## Limitations

- Reporters are NOT available in the editor workflow.
- Preprocessors are supported only via `lsp.knip.settings` and require a Managed Install.
- Alternate TS configuration is handled via `ts_config_path` (Managed Install only) instead of launch arguments.
- `lsp.knip.binary.arguments` is rejected. The Knip language server only recognizes transport flags (e.g., `--stdio`).
- Removed settings such as explicit server path overrides in settings, log levels, or package manager overrides are rejected.

## Development

### Build and Test

```sh

## Run tests

mise x -- cargo nextest run --no-tests=pass

## Run all targets and features

mise x -- cargo nextest run --all-targets --all-features

## Build for WASM

mise x -- cargo build --release --target wasm32-wasip2
```

### Formatting and Linting

```sh

## Format all files

aube run format

## Lint all files

aube run lint
```

### Architecture

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

### Upstream Project

For more information about Knip itself, visit the [official repository](https://github.com/webpro-nl/knip).

### License

MIT
