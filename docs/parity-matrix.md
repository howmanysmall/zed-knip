# Knip VSCode vs Zed Parity Matrix

This document maps every feature of the Knip VSCode extension to its Zed equivalent, implementation status, and limitations.

## Guardrails

- **MUST NOT modify upstream Knip**: All changes for Zed support must be contained within this repository or
  communicated to the user for approval. We do not push changes to the upstream Knip core without explicit request.
- **MUST NOT silently auto-install**: Any dependency installation (e.g., via code actions) must be triggered by a
  user action and visible to the user.
- **MUST NOT submit registry PR**: Do not automatically submit PRs to the Zed extension registry or any other package registry.

## Feature Matrix

Statuses marked **harness-confirmed** are validated by `tests/lsp_parity.rs` (run: `cargo nextest run -E 'test(lsp_parity)'`).

| Feature | VSCode Reference | Zed Equivalent | Status | Harness-Confirmed | Limitations/Notes |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **Diagnostics** | | | | | |
| Unused files | `diagnostics.js` | Diagnostics | `implemented` | ✓ | Published via LSP. |
| Unused dependencies | `diagnostics.js` | Diagnostics | `implemented` | ✓ | Published via LSP. |
| Unused exports | `diagnostics.js` | Diagnostics | `implemented` | ✓ | Published via LSP. |
| Circular dependencies | `diagnostics.js` | Diagnostics | `implemented` | ✓ | Published via LSP. |
| **Code Actions** | | | | | |
| Remove export/type | `code-actions.js` | Code Action | `implemented` | ✓ | LSP `textDocument/codeAction`. |
| Add JSDoc tag | `code-actions.js` | Code Action | `implemented` | ✓ | LSP `textDocument/codeAction`. |
| Remove dependency | `code-actions.js` | Code Action | `implemented` | ✓ | LSP `textDocument/codeAction`. |
| Install dependency | `code-actions.js` | Slash Command | `zed-equivalent` | ✓ | Zed lacks interactive quick-fixes with params; use `/knip-install`. |
| Delete file | `code-actions.js` | Code Action | `unsupported` | ✓ | Zed LSP client does not support file-deletion workspace edits. |
| **Settings** | | | | | |
| `knip.deferSession` | `package.json` | `settings.json` | `implemented` | ✓ | |
| `knip.requireConfig` | `package.json` | `settings.json` | `implemented` | ✓ | |
| `knip.configFilePath` | `package.json` | `settings.json` | `implemented` | ✓ | |
| `knip.nodeRuntimePath` | `package.json` | `settings.json` | `implemented` | ✓ | |
| `knip.preprocessor` | `package.json` | `settings.json` | `implemented` | ✓ | Managed Install only. |
| `knip.preprocessorOptions` | `package.json` | `settings.json` | `implemented` | ✓ | Managed Install only. |
| **Commands** | | | | | |
| `knip.start` | `index.js` | Slash Command | `zed-equivalent` | ✓ | Map to `/knip-start`. |
| `knip.restart` | `index.js` | Slash Command | `zed-equivalent` | ✓ | Map to `/knip-restart`. |
| `knip.showHover` | `index.js` | Hover | `zed-equivalent` | ✓ | Standard Zed hover interaction. |
| `knip.expandAll` | `index.js` | N/A | `unsupported` | ✓ | Zed lacks tree-view command parity. |
| **Hover** | | | | | |
| Export usages | `render-export-hover.js` | Hover | `implemented` | ✓ | LSP `textDocument/hover`. |
| Dependency usages | `render-dependency-hover.js` | Hover | `implemented` | ✓ | LSP `textDocument/hover` on `package.json`. |
| **CodeLens** | | | | | |
| Import counts | `index.js` | Slash Command | `zed-equivalent` | ✓ | Map to `/knip-report` (Zed lacks CodeLens). |
| **Tree Views** | | | | | |
| Imports View | `tree-view-imports.js` | Slash Command | `zed-equivalent` | ✓ | Map to `/knip-imports` markdown report. |
| Exports View | `tree-view-exports.js` | Slash Command | `zed-equivalent` | ✓ | Map to `/knip-exports` markdown report. |
| **Lifecycle** | | | | | |
| LS Startup | `index.js`, `server.js` | Extension Startup | `implemented` | ✓ | Standard LSP lifecycle. |
| File Watching | `index.js` | File System Watcher | `implemented` | ✓ | LSP `workspace/didChangeWatchedFiles`. |
| **MCP** | | | | | |
| `knip-configure` | `package.json` | N/A | `MCP-excluded` | ✓ | MCP features are out of scope for Zed extension. |
| `knip-docs` | `package.json` | N/A | `MCP-excluded` | ✓ | MCP features are out of scope for Zed extension. |
| `languageModelTools` | `package.json` | N/A | `MCP-excluded` | ✓ | MCP features are out of scope for Zed extension. |
| **Custom LSP Methods** | | | | | |
| `knip/openFile` | `constants.js` | Standard file-open | `zed-equivalent` | ✓ | Zed handles via standard file-open mechanism. |
| `knip/showReferences` | `constants.js` | References UI | `zed-equivalent` | ✓ | Zed surfaces via standard references UI. |

## Parity Summary

| Status | Count |
| :--- | :--- |
| `implemented` | 15 |
| `zed-equivalent` | 2 |
| `unsupported` | 9 |
| `MCP-excluded` | 3 |

## UI Limitations

### Tree View

Zed has no tree-view extension API. VSCode's Imports View and Exports View are surfaced as markdown reports via slash
commands (`/knip-imports`, `/knip-exports`). The `knip.expandAll` command has no equivalent.

### CodeLens

Zed does not support `textDocument/codeLens`. Import counts are surfaced via `/knip-report` slash command instead.

### File Deletion

Zed's LSP client does not support `WorkspaceEdit.documentChanges` entries of type `DeleteFile`. The "Delete file" code
action is therefore unsupported.

## Diagnostic Codes

All Knip diagnostics carry `source = "knip"` and a string `code` field. Zed surfaces these in the editor gutter and
problem panel. Severity is always `Warning` (LSP severity 2).

| Code | Diagnostic | Target File |
| :--- | :--- | :--- |
| `unused-file` | Unused file | The unused file itself (range `(0,0)-(0,0)`) |
| `unused-export` | Unused export `'<name>'` | File containing the export (range spans identifier) |
| `unused-dependency` | Unused dependency `'<name>'` | `package.json` (range spans the dependency key) |
| `circular-dependency` | Circular dependency: `a.ts → b.ts → a.ts` | First file in the cycle (range `(0,0)-(0,0)`) |

Multiple diagnostics for the same file are batched into a single `textDocument/publishDiagnostics` notification.
Clearing diagnostics sends an empty array for the file URI.

## Code Action Kinds

All Knip code actions use the `quickfix.knip.` kind prefix. Zed presents them in the lightbulb menu.

| Kind | Title Pattern | `isPreferred` | Notes |
| :--- | :--- | :--- | :--- |
| `quickfix.knip.removeExport` | `Remove export '<name>'` | `true` | Primary fix for `unused-export` |
| `quickfix.knip.addJsDocTag` | `Add @public JSDoc tag` | `false` | Suppress action; marks export intentional |
| `quickfix.knip.removeDependency` | `Remove dependency '<name>'` | `false` | Destructive; user must choose explicitly |

## Hover Shape

Knip hover responses use `MarkupContent` with `kind = "markdown"`. The hover range spans the hovered identifier so
Zed highlights it while the popup is shown.

| Hover Type | Trigger | Content Pattern |
| :--- | :--- | :--- |
| Export usages | Any exported symbol | `**<name>** — used in N files\n\n- \`file\`` |
| Dependency usages | `package.json` dependency key | `**<pkg>** — used in N files\n\n- \`file\`` |
| Unused symbol | Exported symbol with 0 usages | `**<name>** — used in 0 files` |

Hover provider implements a 300ms timeout to prevent UI lag (performance constraint, not protocol).

## Performance Constraints

- **Single Session**: One Knip session per workspace to avoid OOM.
- **Lazy Loading**: Module graph building is deferred if `deferSession` is enabled.
- **Hover Timeouts**: Hover provider implements timeouts (default 300ms) to prevent UI lag.
