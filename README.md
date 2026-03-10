# zed-ctrmml

Zed extension + LSP for ctrmml.

If you prefer VS Code, see https://github.com/ulalume/vscode-ctrmml

> ⚠️ **Early Development**: This project is in active development and features may be incomplete.

## Features

- Tree-sitter syntax highlighting for MML.
- LSP completions (metadata, commands, platform values, PCM paths).
- FM instrument completion: auto-scan workspace for instrument files (.dmp, .fui, .fur, .gin, .ginpkg, etc.) and insert FM parameters as MML.
- Code Actions: play, play from cursor, stop, export vgm/wav, mdslink, quickrom.

## Usage

- Code Actions: macOS `Cmd + .`, Windows/Linux `Ctrl + .`.
- Run commands from the Code Actions list in an `.mml` file.

## Use in Zed (local dev)

1. In Zed, add this repo as a dev extension:
   - Command Palette: "Extensions: Install Dev Extension" and select this folder.
2. Reload Zed.

## Dependencies

- tree-sitter: https://github.com/ulalume/tree-sitter-ctrmml
- language-server: https://github.com/ulalume/language-server-ctrmml
- cmd: https://github.com/ulalume/ctrmml-cmd
- ym2612_format: https://github.com/ulalume/ym2612_format

## License

MIT
