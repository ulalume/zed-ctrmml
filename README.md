# zed-ctrmml

Zed extension + LSP for ctrmml.

## Build (local dev)

From this repo:

```sh
# Build the extension (WASM)
cargo build
```

## Use in Zed (local dev)

1. In Zed, add this repo as a dev extension:
   - Command Palette: "Extensions: Install Dev Extension" and select this folder.
2. Reload Zed.

## Dependencies

- tree-sitter: https://github.com/ulalume/tree-sitter-ctrmml
- language-server: https://github.com/ulalume/language-server-ctrmml
