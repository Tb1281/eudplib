# Zed smoke test: `eudkit-lsp` canonical units

This walks through a dev-extension install and verifies the two things that
matter:

1. Zed starts the shared `eudkit-lsp` for Python.
2. `KillUnit("` shows exactly the 228 canonical unit names.

It also covers the release-download path, version pinning, cache layout, and
mandatory checksum verification.

## Prerequisites

- Zed with extension support. Current Zed builds dev extensions for
  `wasm32-wasip2`; the manual build check below also uses `wasm32-wasip1`.
- Rust installed through `rustup` for the dev-extension WASM build. The
  downloaded `eudkit-lsp` binary itself does not need Rust.
- Run the commands from the repository root unless a step says otherwise.

```sh
rustup target add wasm32-wasip2 wasm32-wasip1
```

## 1. Build the shared language server

```sh
cd src/rust
cargo build -p eudkit-lsp --release
cd ../..
```

The binary is:

- Linux/macOS: `src/rust/target/release/eudkit-lsp`
- Windows: `src/rust/target/release/eudkit-lsp.exe`

Confirm the version contract:

```sh
src/rust/target/release/eudkit-lsp --version
# eudkit-lsp 0.1.0
```

Verify the LSP itself returns all 228 units before involving Zed. The
repository script works on Linux, macOS, and Windows:

```sh
python scripts/smoke_eudkit_lsp.py \
  src/rust/target/release/eudkit-lsp --expected-version 0.1.0
```

The last lines should read:

```text
... reports eudkit-lsp 0.1.0
eudkit-lsp returned 228 canonical units
eudkit-lsp smoke test passed for ...
```

## 2. Install the dev extension

1. Open the command palette in Zed.
2. Run `zed: extensions`.
3. Click **Install Dev Extension**.
4. Select the `editors/zed` directory (the one containing `extension.toml`).
5. Wait for Zed to compile the extension. The Extensions page should show it
   as a dev extension, and `editors/zed/extension.wasm` should appear.

If compilation fails, inspect `zed: open log`; the build runs
`cargo build --target wasm32-wasip2` in `editors/zed`.

You can also use the published source archive
`eudkit-zed-0.1.0.tar.gz` from a GitHub Release. Extract it and select the
extracted `eudkit-zed-0.1.0` directory. It contains `extension.toml`,
`Cargo.toml`, `Cargo.lock`, `src/`, `README.md`, `TESTING.md`, and `LICENSE`,
with no `target/` or `extension.wasm`.

## 3. Use the local binary override

The extension reads `EUDKIT_LSP_PATH` from the shell environment Zed sees
(`worktree.shell_env()`), not from `std::env` inside WASM. Set it where Zed is
launched.

Linux/macOS:

```sh
export EUDKIT_LSP_PATH="$PWD/src/rust/target/release/eudkit-lsp"
zed --foreground
```

Windows PowerShell:

```powershell
$env:EUDKIT_LSP_PATH = "$PWD\src\rust\target\release\eudkit-lsp.exe"
zed --foreground
```

If Zed is launched from a desktop icon instead of a terminal, set
`EUDKIT_LSP_PATH` as a user/system environment variable and restart Zed.
Changes to the variable are only picked up on restart.

With `--foreground`, the extension prints:

```text
eudkit-lsp: using EUDKIT_LSP_PATH=/absolute/path/to/eudkit-lsp
```

That confirms the first entry in the resolution order won.

## 4. Completion smoke test in Zed

1. Open `editors/zed/fixtures/kill_unit.py`.
2. Delete `Terran Marine` from the `KillUnit` call so the line reads
   `KillUnit("", player)` and place the cursor between the quotes. To reproduce
   the literal acceptance input, also delete the closing quote so the line ends
   with `KillUnit("`.
3. If the completion menu is not already open, run
   `editor: completions` (usually <kbd>ctrl-space</kbd>).
4. Confirm the list contains canonical names such as `Terran Marine`,
   `Zerg Zergling`, and `Protoss Zealot`.
5. Confirm the list does **not** contain stock filters such as `Any unit`,
   `Men`, `Buildings`, or `Factories`, and does not contain custom `UNI0` /
   `UNI1` / `UNIx` names.
6. Type `Mar` and confirm filtering still returns `Terran Marine`; then delete
   it and trigger completion again to see the full list.

The acceptance check is the full, unfiltered list immediately after
`KillUnit("`: it must contain exactly 228 entries. The standalone LSP check in
step 1 asserts the count; Zed renders the same response.

## 5. Server logs

Run Zed with `--foreground` to see extension `stdout` and LSP lifecycle logs,
or use `zed: open log` after the fact. Look for:

- `eudkit-lsp: using ...` or `eudkit-lsp: downloading ...`
- `eudkit-lsp: verified SHA-256 checksum for ...`
- `eudkit-lsp: installed ...`
- A language server start for `eudkit-lsp` when the Python buffer opens
- No `failed to download`, `checksum`, or `unsupported platform` errors

If the binary cannot be started, Zed marks the language server as failed and
the extension returns an error naming the asset and checksum URLs and the
`EUDKIT_LSP_PATH` / `PATH` overrides.

## 6. Release-download path and mandatory checksum

Use this after a release containing the `eudkit-lsp-*` assets exists. The
extension package version is `0.1.0`, so the download is pinned to tag
`v0.1.0` and the cache directory `eudkit-lsp-0.1.0/`.

1. Remove the local override and make sure no `eudkit-lsp` is on `PATH`:

   ```sh
   unset EUDKIT_LSP_PATH
   which eudkit-lsp   # should print nothing
   ```

   On Windows use `Remove-Item Env:EUDKIT_LSP_PATH` and `where eudkit-lsp`.

2. Delete the cache so the download runs again:

   ```sh
   rm -rf ~/.local/share/zed/extensions/work/eudkit-lsp
   # macOS: ~/Library/Application\ Support/Zed/extensions/work/eudkit-lsp
   # Windows: %LOCALAPPDATA%\Zed\extensions\work\eudkit-lsp
   ```

3. Restart Zed with `--foreground` and open a Python file. The log should show
   a download from:

   ```text
   https://github.com/Tb1281/eudplib/releases/download/v0.1.0/<asset>
   ```

   followed by `verified SHA-256 checksum` and `installed`. The version in the
   URL comes from `CARGO_PKG_VERSION`; it is not read from the downloaded
   binary because zed_extension_api 0.7 cannot execute arbitrary commands.

4. Inspect the cache. It contains the platform asset plus its `.sha256`
   companion, for example on Linux:

   ```text
   ~/.local/share/zed/extensions/work/eudkit-lsp/eudkit-lsp-0.1.0/eudkit-lsp-linux-x86_64
   ~/.local/share/zed/extensions/work/eudkit-lsp/eudkit-lsp-0.1.0/eudkit-lsp-linux-x86_64.sha256
   ```

5. Verify the checksum manually:

   ```sh
   cd ~/.local/share/zed/extensions/work/eudkit-lsp/eudkit-lsp-0.1.0
   sha256sum -c eudkit-lsp-linux-x86_64.sha256
   # or: sha256sum eudkit-lsp-linux-x86_64 && cat eudkit-lsp-linux-x86_64.sha256
   ```

   On macOS use `shasum -a 256 -c ...`.

6. Confirm the pinning and checksum helpers with the host tests:

   ```sh
   cargo test pins_download_url_and_cache_to_the_package_version
   cargo test missing_checksums_are_reported_as_required
   cargo test reports_checksum_mismatch
   ```

7. To exercise a missing checksum end to end, publish a temporary asset
   without its `.sha256` companion (or point `RELEASE_REPOSITORY` /
   `RELEASE_VERSION` at a controlled test release) and repeat step 3. The
   extension must report `missing its required .sha256 companion`, install no
   executable, and leave the cache empty. Restore the normal release settings
   afterward.

There is no fallback for a missing checksum: every download must verify its
published SHA-256 companion.

## 7. No map-path setting

The extension intentionally has no map-path setting. Confirm with:

```sh
rg -n "map[_-]?path|mapPath" editors/zed/src editors/zed/extension.toml
```

There should be no matches. `eudkit-lsp` does not need a map path for canonical
unit completion, and the Zed extension must not invent a Zed-only setting.

## Troubleshooting

- **Completion list is empty**: confirm the buffer language is Python, the
  extension is installed, and `KillUnit("` is the first argument. Check
  `zed: open log` for the language server start line.
- **Extension says unsupported platform**: only Windows x86_64, Linux x86_64,
  macOS arm64, and macOS x86_64 assets are published. Set `EUDKIT_LSP_PATH` to
  a locally built binary.
- **`EUDKIT_LSP_PATH` is ignored**: launch Zed from the shell where the
  variable is exported, or set it at the OS level and restart Zed.
- **Download fails**: the matching release assets may be missing or the network
  is offline. Use the local override until the release is available.
- **Checksum failure**: the asset and `.sha256` companion do not match, or the
  companion is missing. Re-publish the release assets together; do not copy a
  binary into the cache manually.
- **Version mismatch**: the extension is pinned to its own package version.
  Install the extension release that matches the `eudkit-lsp` release series.
- **Rebuilding after Rust changes**: reinstall the dev extension after editing
  `editors/zed`; Zed recompiles the WASM. The `eudkit-lsp` binary itself is a
  separate build and can be rebuilt independently.
