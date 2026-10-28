# EUDKIT LSP for Zed

Zed extension that registers the shared [`eudkit-lsp`](../../src/rust/eudkit-lsp)
language server for Python. It adds completion for the 228 canonical StarCraft
unit names inside `KillUnit("...")`.

The extension contains no editor-specific completion logic. Zed and VS Code
both talk to the same `eudkit-lsp` binary, so they see the same capabilities,
the same 228-entry unit table, and the same future features.

## What it provides

- The `eudkit-lsp` language server for `Python` documents.
- Canonical unit completion inside the first `KillUnit` argument, for example
  `KillUnit("` or `KillUnit("Mar`.
- No map-path setting. `eudkit-lsp` does not need a map path, and neither does
  this extension.

## Installing

### Development build

See [TESTING.md](TESTING.md) for the complete Zed smoke test, including the
local binary override and server logs.

1. Install the Rust targets Zed may use:

   ```sh
   rustup target add wasm32-wasip2 wasm32-wasip1
   ```

   Current Zed builds dev extensions for `wasm32-wasip2`. The
   `wasm32-wasip1` target is also kept because this extension's original
   build check used it, and older Zed releases built extensions for that
   target.

2. Build `eudkit-lsp`:

   ```sh
   cargo build -p eudkit-lsp --release
   ```

   Run that from `src/rust`, or use
   `cargo build --manifest-path src/rust/Cargo.toml -p eudkit-lsp --release`
   from the repository root. The binary is
   `src/rust/target/release/eudkit-lsp` (`.exe` on Windows).

3. In Zed, run `zed: extensions`, click **Install Dev Extension**, and select
   the `editors/zed` directory. Zed compiles the extension to WebAssembly.

4. Point the extension at the binary while releases do not contain LSP assets
   yet:

   ```sh
   export EUDKIT_LSP_PATH="$PWD/src/rust/target/release/eudkit-lsp"
   ```

   Launch Zed from the same environment (or set the variable at the OS level),
   then open a Python file and type `KillUnit("`.

### Release binary

If `EUDKIT_LSP_PATH` is unset and `eudkit-lsp` is not on `PATH`, the extension
downloads the release asset for the current platform from:

```text
https://github.com/Tb1281/eudplib/releases/download/v0.1.0/<asset>
```

The version comes from this package's version (`0.1.0`) and must match the
GitHub release tag. The published `.sha256` companion is downloaded and
verified when it exists. If the checksum asset is missing, the download is
accepted with a warning in the Zed log. On macOS and Linux the binary is made
executable.

Downloads are cached in the extension work directory (not in this repository):

- Linux: `~/.local/share/zed/extensions/work/eudkit-lsp/`
- macOS: `~/Library/Application Support/Zed/extensions/work/eudkit-lsp/`
- Windows: `%LOCALAPPDATA%\Zed\extensions\work\eudkit-lsp\`

The exact cache layout is `eudkit-lsp-0.1.0/<asset>`.

## Binary resolution order

1. `EUDKIT_LSP_PATH` (development override).
2. `eudkit-lsp` found on `PATH` through `worktree.which`.
3. The cached release download, if present.
4. A fresh GitHub release download followed by checksum verification.

If none of these work, Zed shows an error that names the failed asset URL and
explains how to set `EUDKIT_LSP_PATH` or put `eudkit-lsp` on `PATH`.

## Release assets

| Platform | Architecture | Asset |
| --- | --- | --- |
| Windows | x86_64 | `eudkit-lsp-windows-x86_64.exe` |
| Linux | x86_64 | `eudkit-lsp-linux-x86_64` |
| macOS | arm64 | `eudkit-lsp-macos-aarch64` |
| macOS | x86_64 | `eudkit-lsp-macos-x86_64` |

Other combinations return an explicit unsupported-platform error rather than
guessing an asset name. Each asset is accompanied by `<asset>.sha256`.

## Building and testing the extension

From `editors/zed`:

```sh
cargo test
cargo build --target wasm32-wasip1 --release
cargo build --target wasm32-wasip2 --release
```

The extension crate is standalone; it is not part of the `src/rust` workspace.
Nothing from `target/` or the extension work directory is committed.
