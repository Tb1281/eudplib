# eudkit LSP for VS Code

This extension connects VS Code's Python, TypeScript, and JavaScript editors to
[`eudkit-lsp`](../../src/rust/eudkit-lsp), the eudplib language server. For the
end-to-end user guide, including first run, Map Context discovery, diagnostics,
and troubleshooting, see
[`docs/eudkit-lsp-user-guide.md`](../../docs/eudkit-lsp-user-guide.md). It adds
completion for the 228 canonical unit names in the `KillUnit` unit argument, for
example after typing `KillUnit("`.

The extension is intentionally narrow:

- It activates for Python, TypeScript, TSX, JavaScript, and JSX documents and
  starts `eudkit-lsp` as a language client.
- It has no map-path setting and never asks for a map. `eudkit-lsp` does not
  need one.
- It does not synthesize type errors. Pylance (Python) and tsserver
  (TypeScript/JavaScript) remain the source of API types and type diagnostics;
  the eudkit client runs alongside them.
- eudkit's own Map Symbol and EUD warnings are published as LSP warnings and
  appear in the Problems panel. They are advisory and never block
  `eudkit build`.

## Requirements

- VS Code 1.91 or newer.
- Windows x64, Linux x64, macOS arm64, or macOS x86_64.
- No Rust toolchain. On first use the extension downloads the matching
  published `eudkit-lsp` binary, verifies its SHA-256 checksum, and checks its
  `--version` output. You can also point it at a locally built server with
  `eudkit.lsp.path` or `EUDKIT_LSP_PATH`.

## Server resolution

The extension uses the first usable source:

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
5. A GitHub Releases download for the current platform and architecture. The
   VSIX is platform-neutral, so this is the normal path for a clean install.

Every resolved binary is asked for `eudkit-lsp --version`. The compatibility
rule is:

> Extension `0.1.x` accepts only `eudkit-lsp` `0.1.x`. Patch releases stay
> compatible within a series; `0.2.x` requires a matching extension update.

A version mismatch is an actionable error, not a silent start.

### Release downloads

Downloads are pinned to the extension's own package version. An `0.1.1`
extension fetches:

```text
https://github.com/Tb1281/eudplib/releases/download/v0.1.1/<asset>
```

The published `<asset>.sha256` companion is mandatory. The extension downloads
it first, verifies the asset's SHA-256 hash, caches both files under VS Code's
global storage, and only then runs `--version`. A missing checksum, a checksum
mismatch, a 404, or an offline network fails with an error naming the URL and
the local override options. On Unix the cached binary is made executable.

Published assets:

| Platform | Architecture | Asset |
| --- | --- | --- |
| Windows | x86_64 | `eudkit-lsp-windows-x86_64.exe` |
| Linux | x86_64 | `eudkit-lsp-linux-x86_64` |
| macOS | arm64 | `eudkit-lsp-macos-aarch64` |
| macOS | x86_64 | `eudkit-lsp-macos-x86_64` |

Other platform/architecture pairs fail with an explicit unsupported-platform
message. Set `eudkit.lsp.path`, set `EUDKIT_LSP_PATH`, or put a locally built
`eudkit-lsp` on `PATH` for those combinations.

Cache layout (inside VS Code's global storage directory for the extension):

```text
eudkit-lsp/<extension-version>/<asset>
eudkit-lsp/<extension-version>/<asset>.sha256
```

If a cached binary fails its version check it is discarded and downloaded
again. All attempted sources, URLs, checksums, and errors are written to the
**eudkit LSP** output channel.

If no source works, the extension shows an error with actions to open the
setting, show the output channel, or retry.

## Settings

| Setting | Default | Description |
| --- | --- | --- |
| `eudkit.lsp.path` | `""` | Path to the `eudkit-lsp` executable. Absolute or relative to the first workspace folder. |

Changing this setting restarts the language server automatically. There is no
`eudkit.inputMap` or other map-path setting.

## Commands

- **eudkit: Restart Language Server** (`eudkit.restartLanguageServer`) stops and
  starts the client with the current settings.

Server stderr and lifecycle events are written to the **eudkit LSP** output
channel. If the server crashes, the extension reports it and offers a
**Restart** action.

## Install smoke test

See [`docs/editor-install-smoke-test.md`](../../docs/editor-install-smoke-test.md)
for the clean-machine check: install the VSIX without Rust, let the extension
download and verify the server, then confirm the first `KillUnit("` completion.

## Development

From this directory:

```bash
npm ci
npm run check       # strict TypeScript check, no emit
npm test            # unit tests + tsc/checkJs type fixtures
npm run test:types  # tsc --noEmit for the TypeScript and checkJs fixtures
npm run build       # esbuild bundle in dist/extension.js
npm run package     # VSIX under dist/
npm run verify:vsix # inspect the built VSIX (requires unzip)
```

The type fixtures import `KillUnit` from `packages/ts-sdk` through a `paths`
mapping. The TypeScript fixture verifies `tsc --noEmit`; the JavaScript fixture
uses `checkJs` and JSDoc. The VSIX is bundled with esbuild and does not include
`node_modules`, sources, or tests. See [TESTING.md](TESTING.md) for the
Extension Development Host smoke test and release-download checks.

## License

MIT. See [LICENSE](LICENSE); the repository root also contains the canonical
[eudplib license](../../LICENSE).
