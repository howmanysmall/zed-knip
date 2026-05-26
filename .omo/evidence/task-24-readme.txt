=== README Local Install Instructions Check ===

14:- **Managed Installation**: Automatically downloads and manages the Knip language server if it's not found in your workspace.
23:## Installation
25:### Local Development Install

=== Install section content ===
### Local Development Install

1. Build the extension WASM artifact:

   ```sh
   mise x -- cargo build --release --target wasm32-wasip1
   ```

2. Copy the extension directory to your local Zed extensions folder:

   - **macOS**: `~/Library/Application Support/Zed/extensions/zed-knip/`
   - **Linux**: `~/.local/share/zed/extensions/zed-knip/`

   The directory should contain `extension.toml` and the compiled `extension.wasm` (rename `target/wasm32-wasip1/release/zed_knip.wasm` to `extension.wasm`).

3. Restart Zed to load the extension.

## Settings

Configure the extension in your Zed `settings.json`:

