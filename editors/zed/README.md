# EUDKIT LSP for Zed

Zed extension that registers the shared [`eudkit-lsp`](../../src/rust/eudkit-lsp)
language server for Python, TypeScript, TSX, and JavaScript. It adds completion
for the 228 canonical StarCraft unit names inside `KillUnit("...")`, hover and
signature help for supported eudkit calls and canonical/Map Context symbols,
and exposes the server's Map Context and warning behavior in all four languages.
For the end-to-end user guide, including installation, first run, Map Context
discovery, diagnostics, hover, signature help, and troubleshooting, see
[`docs/eudkit-lsp-user-guide.md`](../../docs/eudkit-lsp-user-guide.md).

The extension contains no editor-specific completion logic. Zed and VS Code
both talk to the same `eudkit-lsp` binary, so both editors receive the same LSP
capabilities and responses, including the same 228-entry unit table. Zed's
rendering of those responses was not independently driven; see the
[acceptance matrix](../../docs/eudkit-lsp-acceptance-matrix.md) for the verified
scope.

## What it provides

- The `eudkit-lsp` language server for `Python`, `TypeScript`, `TSX`, and
  `JavaScript` documents. Zed's `JavaScript` language also covers `.jsx`
  files; Zed has no separate JSX language name.
- Canonical unit completion inside the first `KillUnit` argument, for example
  `KillUnit("` or `KillUnit("Mar`.
- Map Context location and switch completion when an `eudkit.toml` is found at
  or above the document. Without one, the shared server falls back to the
  canonical `Anywhere` location and the canonical switch names.
- Hover for canonical and Map Context unit, location, and switch strings,
  including the symbol's canonical/map status and ID. Hover and signature help
  for the supported eudkit calls include the signature, summary, return type,
  and symbol-accepting parameters.
- Signature help in Python, TypeScript, and JavaScript, including Python keyword
  arguments and qualified `eudkit.KillUnit(...)` calls.
- Unknown Map Symbol and EUD player warnings from the shared server in Zed's
  diagnostics. Stock Unit Filters such as `Any unit`, `Men`, `Buildings`, and
  `Factories` do not produce false positives.
- No map-path setting and no `eudkit.inputMap` setting. `eudkit-lsp` discovers
  `eudkit.toml` from the document path, and neither the server nor this
  extension asks for a map path.
- Pylance and tsserver remain the sources of general API types and type
  diagnostics; eudkit warnings appear alongside them with source `eudkit`.
  eudkit hover and signature help are deliberately limited to canonical/Map
  Context symbols and the supported call list.

Hover and signature help shipped in `v0.1.2`. The older
`eudkit-zed-0.1.1.tar.gz` archive does not contain them; install `v0.1.2` or
later to get them.

## Feature parity checklist

The shared server owns all behavior; this extension only registers it for the
Zed languages. Run the detailed manual checklist in [TESTING.md](TESTING.md)
after changing the manifest or the server.

- [ ] Python: `KillUnit("` offers canonical units; `Bring(..., "` and
      `SetSwitch("` offer Map Context symbols.
- [ ] TypeScript: the same completion behavior in `.ts` files.
- [ ] TSX: the same behavior in `.tsx` files, reported as `typescriptreact`.
- [ ] JavaScript: the same behavior in `.js` files. `.jsx` opens as Zed's
      JavaScript language.
- [ ] With `eudkit.toml`: Map Context locations and named switches are added
      to the canonical symbols.
- [ ] Without `eudkit.toml`: only canonical symbols are offered
      (`Anywhere`, `Switch 1`..`Switch 256`, and the 228 unit names).
- [ ] Unknown Map Symbols warn with source `eudkit`; Stock Unit Filters and
      numeric stock IDs do not.
- [ ] Hover a canonical unit/location/switch string and a supported call; the
      server returns its name, status, ID, or call documentation.
- [ ] Signature help opens inside `KillUnit(` and advances after a comma;
      `eudkit.KillUnit(...)` works in TypeScript and JavaScript.
- [ ] EUD group-player warnings appear for offsets above 232 and stay silent
      for stock units, concrete players, variables, and ambiguous expressions.
- [ ] No `eudkit.inputMap` or map-path setting exists.

See [`docs/editor-install-smoke-test.md`](../../docs/editor-install-smoke-test.md)
for the clean-machine install check. Terms such as Map Context, Map Symbol, and
Stock Unit Filter are defined in [`GLOSSARY.md`](../../GLOSSARY.md).

## Installing

### Release source package

GitHub Releases publish a dev-extension source archive named
`eudkit-zed-0.1.2.tar.gz`. It contains `extension.toml`, `Cargo.toml`,
`Cargo.lock`, `src/`, `README.md`, `TESTING.md`, and `LICENSE`; build output
such as `target/` and `extension.wasm` is excluded.

1. Extract the archive.
2. In Zed, run `zed: extensions`, click **Install Dev Extension**, and select
   the extracted `eudkit-zed-0.1.2` directory (the one containing
   `extension.toml`).
3. Zed compiles the extension to WebAssembly, which requires a Rust toolchain
   with the `wasm32-wasip2` target **for this dev-extension install step only**.
   The `eudkit-lsp` server itself is downloaded as a native binary and does not
   need Rust.
4. Open a Python, TypeScript, TSX, or JavaScript file and type `KillUnit("`.

A later Zed registry package will remove the local Rust build step; the source
archive is deliberately shaped so it can be submitted unchanged.

### Development build from this repository

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

4. Optionally point the extension at the locally built binary while developing:

   ```sh
   export EUDKIT_LSP_PATH="$PWD/src/rust/target/release/eudkit-lsp"
   ```

   Launch Zed from the same environment (or set the variable at the OS level),
   then open a Python, TypeScript, TSX, or JavaScript file and type `KillUnit("`.

## Release binary and version pinning

If `EUDKIT_LSP_PATH` is unset and `eudkit-lsp` is not on `PATH`, the extension
downloads the release asset for the current platform from:

```text
https://github.com/Tb1281/eudplib/releases/download/v0.1.2/<asset>
```

The version is pinned at compile time from this package's `CARGO_PKG_VERSION`
(`0.1.2`). It **must** match the GitHub release tag. The cache directory is also
version-scoped:

```text
eudkit-lsp-0.1.2/<asset>
```

### Why there is no runtime `--version` check

The Zed extension API 0.7 does not let an extension execute arbitrary commands,
so the Zed extension cannot run `eudkit-lsp --version` the way the VS Code
extension does. Version compatibility is instead enforced structurally:

- the download URL is pinned to the extension's own release tag
  (`0.1.2` → `v0.1.2`);
- the cache is keyed by that same version, so a different extension version
  never reuses another version's binary;
- the SHA-256 companion is required and verified before the binary is moved
  into the cache;
- the extension only supports the same `0.1.x` release series as its package
  version. Installing a different server series requires installing the
  matching extension version.

Do not edit the version directory or copy a binary from another release into
it manually.

### Checksums are mandatory

Every binary download must have a published `<asset>.sha256` companion. If the
companion is missing, unreadable, malformed, or reports a different hash, the
download is rejected and no executable is installed. There is no
"checksum when available" fallback.

On macOS and Linux the verified binary is made executable.

Downloads are cached in the extension work directory (not in this repository):

- Linux: `~/.local/share/zed/extensions/work/eudkit-lsp/`
- macOS: `~/Library/Application Support/Zed/extensions/work/eudkit-lsp/`
- Windows: `%LOCALAPPDATA%\Zed\extensions\work\eudkit-lsp\`

The exact layout is `eudkit-lsp-0.1.2/<asset>` plus
`eudkit-lsp-0.1.2/<asset>.sha256`.

## Binary resolution order

1. `EUDKIT_LSP_PATH` (development override).
2. `eudkit-lsp` found on `PATH` through `worktree.which`.
3. The cached release download, if present.
4. A fresh GitHub release download followed by mandatory checksum verification.

If none of these work, Zed shows an error that names the failed asset and
checksum URLs and explains how to set `EUDKIT_LSP_PATH` or put `eudkit-lsp` on
`PATH`.

## Release assets

| Platform | Architecture | Asset |
| --- | --- | --- |
| Windows | x86_64 | `eudkit-lsp-windows-x86_64.exe` |
| Linux | x86_64 | `eudkit-lsp-linux-x86_64` |
| macOS | arm64 | `eudkit-lsp-macos-aarch64` |
| macOS | x86_64 | `eudkit-lsp-macos-x86_64` |

Other combinations return an explicit unsupported-platform error rather than
guessing an asset name. Each asset is accompanied by `<asset>.sha256`.

The release also publishes `BINARY-SIZES.txt`, which records the size of the
lean `eudkit` build CLI and `eudkit-lsp` for every platform.

## Building and testing the extension

From `editors/zed`:

```sh
cargo test
cargo build --target wasm32-wasip1 --release
cargo build --target wasm32-wasip2 --release
```

The extension crate is standalone; it is not part of the `src/rust` workspace.
Nothing from `target/` or the extension work directory is committed.
