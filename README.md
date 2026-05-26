# zed-knip

## Local Zed extension install (macOS)

1. Build the extension WASM artifact:

   ```sh
   mise x -- cargo build --release --target wasm32-wasip1
   ```

2. Copy the extension files into Zed's local extensions directory:

   ```sh
   ~/Library/Application Support/Zed/extensions/zed-knip/
   ```

3. Place the built artifact at:

   ```sh
   target/wasm32-wasip1/release/zed_knip.wasm
   ```

4. Restart Zed to load the local extension.
