# eudkit LSP for VS Code

This extension connects VS Code's Python editor to [`eudkit-lsp`](../../src/rust/eudkit-lsp),
the eudplib language server. It adds completion for the 228 canonical unit
names in the `KillUnit` unit argument, for example after typing `KillUnit("`.

The extension is intentionally narrow:

- It activates for Python and starts `eudkit-lsp` as a language client.
- It does not provide or publish type diagnostics. Pylance remains the source
  of API type checking and type errors.
- It has no map-path setting and never asks for a map. `eudkit-lsp` does not
  need one.

## Requirements

- VS Code 1.91 or newer.
- An `eudkit-lsp` executable. The server is built separately from this
  extension; see [TESTING.md](TESTING.md) for the development build commands.

## Server resolution

The extension tries these sources in order and uses the first executable file:

1. The `eudkit.lsp.path` setting. It may be an absolute path or a path relative
   to the first workspace folder.
2. The `EUDKIT_LSP_PATH` environment variable. Relative values are resolved
   the same way as the setting.
3. `eudkit-lsp` on `PATH` (using `PATHEXT` on Windows).
4. A bundled/development binary at
   `bin/<platform>-<arch>/eudkit-lsp` relative to the extension root, or
   `eudkit-lsp.exe` on Windows. For example:
   `bin/linux-x64/eudkit-lsp`, `bin/win32-x64/eudkit-lsp.exe`, or
   `bin/darwin-arm64/eudkit-lsp`.

If no executable is found, the extension shows an error with actions to open
the setting, show the output channel, or retry. Every path that was tried is
logged in the `eudkit LSP` output channel.

## Settings

| Setting | Default | Description |
| --- | --- | --- |
| `eudkit.lsp.path` | `""` | Path to the `eudkit-lsp` executable. Absolute or relative to the first workspace folder. |

Changing this setting restarts the language server automatically.

## Commands

- **eudkit: Restart Language Server** (`eudkit.restartLanguageServer`) stops and
  starts the client with the current settings.

Server stderr and lifecycle events are written to the **eudkit LSP** output
channel. If the server crashes, the extension reports it and offers a
**Restart** action.

## Development

From this directory:

```bash
npm install
npm run check   # strict TypeScript check, no emit
npm test        # server resolution/path unit tests
npm run build   # esbuild bundle in dist/extension.js
npm run package # VSIX under dist/
```

The VSIX is bundled with esbuild and does not include `node_modules`. See
[TESTING.md](TESTING.md) for the Extension Development Host smoke test that
exercises `KillUnit("` completion.

## License

MIT. See [LICENSE](LICENSE); the repository root also contains the canonical
[eudplib license](../../LICENSE).
