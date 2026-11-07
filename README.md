# zed-ctrmml

Zed extension + LSP for ctrmml.

If you prefer VS Code, see https://github.com/ulalume/vscode-ctrmml

> ⚠️ **Early Development**: This project is in active development and features may be incomplete.

## Features

- Tree-sitter syntax highlighting for MML.
- LSP completions: metadata keywords and values (including `#timesig` / `#group`), MML commands, platform values, PCM file paths, and the PCM instrument list at `@N pcm`.
- Chord, dyad, and (opt-in) arpeggio completions on track lines; chord bodies are stacked upward by default.
- Measure fill: typing `|` offers rests to complete the current measure.
- FM instrument completion: auto-scan workspace for instrument files (.dmp, .fui, .fur, .gin, .ginpkg, etc.) and insert FM parameters as MML. Re-picking a patch replaces the previously inserted parameter block.
- Code Actions: play, play from cursor, stop, export vgm/wav, mdslink, quickrom.

## Usage

- Code Actions: macOS `Cmd + .`, Windows/Linux `Ctrl + .`.
- Run commands from the Code Actions list in an `.mml` file.

## Configuration

Completion behavior is configured through Zed's `lsp` settings via `settings`; changes apply without restarting the server. The settings key is the language-server ID `ctrmml` — not "ctrmml LSP", which is only the display name Zed's UI shows for the server.

Example `settings.json` enabling arpeggio completions:

```json
{
  "lsp": {
    "ctrmml": {
      "settings": {
        "arpeggio_enabled": true,
        "arpeggio_pattern": "up"
      }
    }
  }
}
```

| Option | Values | Default | Description |
| --- | --- | --- | --- |
| `arpeggio_enabled` | `bool` | `false` | Enable arpeggio completions on track lines. |
| `arpeggio_pattern` | `"up"` \| `"down"` \| `"updown"` \| `"downup"` \| `"alberti"` (case-insensitive) | `"up"` | Note order for arpeggio completions. |
| `chord_stack_mode` | `"stack_up"` \| `"plain"` (`"stack-up"` accepted as alias) | `"stack_up"` | Voicing used for chord/dyad completion bodies. |
| `fm_picker_hierarchy` | `bool` | `false` | `false`: one item per patch. `true`: two-step file → patch picker. |
| `command_path` | `string` | *(omitted)* | `ctrmml-cmd` to run. Omitted: `CTRMML_CMD_PATH`, then `PATH`, then the release the server downloads. |

- camelCase key spellings (e.g. `arpeggioEnabled`) are also accepted.
- Invalid values are ignored, with a warning logged by the server.
- The same keys are accepted in `initialization_options`; `settings` wins, and a key removed from `settings` falls back to `initialization_options`. After changing `initialization_options`, restart the language server (Command Palette: `editor: restart language server`) or reload Zed.
- `lsp.ctrmml.binary` (`path`, `arguments`, `env`) is honored too, if you want to run your own server binary.

### Completion changes in language server v0.6.9

- Chord and dyad completion bodies now default to the stacked, octave-carrying form (e.g. `f/a/>c`). Set `chord_stack_mode` to `"plain"` to restore the old close voicing (`f/a/c`).
- Meta values (`#platform`, `#option`, `#timesig`, `#group`) insert with an explicit replace range: accepting a suggestion replaces the value token you were typing instead of appending to it.
- Re-picking an FM instrument on an `@N fm` line replaces the previously inserted parameter block instead of leaving a duplicate behind.

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
