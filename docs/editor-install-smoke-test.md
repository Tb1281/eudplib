# Clean-machine editor install smoke test

This is the no-Rust-toolchain check for the published editor artifacts. Run the
primary pass on any supported platform with VS Code and Zed installed: Windows
x64, Linux x64, macOS arm64, or macOS x86_64. The matching native server is
downloaded for whichever platform runs the test; Windows PowerShell examples are
kept below for the reference Windows x64 run.

The goal is to prove that a user can install the extension, let it obtain and
verify `eudkit-lsp`, and get the first canonical-unit completion without a Rust
toolchain. The VS Code half of this document is the no-Rust check. The Zed
artifact is a source package that Zed compiles locally, so the Zed half needs a
Rust toolchain (section 7); run it on a separate machine if the primary machine
must stay Rust-free.

## Artifacts

From <https://github.com/Tb1281/eudplib/releases/tag/v0.1.1>:

- `eudkit-vscode-0.1.1.vsix`
- `eudkit-vscode-0.1.1.vsix.sha256`
- `eudkit-zed-0.1.1.tar.gz` (Zed dev-extension source package)
- `eudkit-zed-0.1.1.tar.gz.sha256`
- `eudkit-lsp-windows-x86_64.exe` + `.sha256`
- `eudkit-lsp-linux-x86_64` + `.sha256`
- `eudkit-lsp-macos-aarch64` + `.sha256`
- `eudkit-lsp-macos-x86_64` + `.sha256`
- `eudkit-<platform>` + `.sha256` for the separate `eudkit` build CLI
- `BINARY-SIZES.txt`

## 1. Verify the downloaded packages (optional but recommended)

Windows PowerShell:

```powershell
Get-FileHash .\eudkit-vscode-0.1.1.vsix -Algorithm SHA256
Get-Content .\eudkit-vscode-0.1.1.vsix.sha256
```

Linux/macOS:

```sh
sha256sum -c eudkit-vscode-0.1.1.vsix.sha256
shasum -a 256 -c eudkit-zed-0.1.1.tar.gz.sha256   # macOS
```

The two hashes must match.

## 2. Confirm this is a clean machine

Windows PowerShell:

```powershell
Get-Command cargo -ErrorAction SilentlyContinue   # should print nothing
Get-Command rustc -ErrorAction SilentlyContinue   # should print nothing
Get-Command eudkit-lsp -ErrorAction SilentlyContinue
Remove-Item Env:EUDKIT_LSP_PATH -ErrorAction SilentlyContinue
```

Linux/macOS:

```sh
command -v cargo || true    # should print nothing
command -v rustc || true
command -v eudkit-lsp || true
unset EUDKIT_LSP_PATH
```

If VS Code has been used for eudkit before, also remove the extension's cached
download and any `eudkit.lsp.path` override:

- Windows: `%APPDATA%\Code\User\globalStorage\eudkit.eudkit-vscode\eudkit-lsp\`
- Linux: `~/.config/Code/User/globalStorage/eudkit.eudkit-vscode/eudkit-lsp/`
- macOS: `~/Library/Application Support/Code/User/globalStorage/eudkit.eudkit-vscode/eudkit-lsp/`

## 3. Install the VSIX

Windows PowerShell:

```powershell
code --install-extension .\eudkit-vscode-0.1.1.vsix
```

Linux/macOS:

```sh
code --install-extension ./eudkit-vscode-0.1.1.vsix
```

Restart VS Code or reload the window. The extensions list should show
**eudkit LSP**. No Rust installation step is involved.

## 4. First completion

1. Create an empty folder with `smoke.py`.
2. Open the folder in VS Code.
3. Open the **eudkit LSP** output channel (Command Palette → **Output: Show
   Output Channel** → `eudkit LSP`).
4. In `smoke.py`, type:

   ```python
   KillUnit("
   ```

5. Trigger completion if needed (<kbd>Ctrl</kbd>+<kbd>Space</kbd>).
6. Confirm the list contains `Terran Marine`, `Zerg Zergling`, and
   `Protoss Zealot`, and does not contain `Any unit`, `Men`, or custom `UNI`
   filters.

The output channel should show a sequence like the following. On Linux or
macOS the asset name is `eudkit-lsp-linux-x86_64`, `eudkit-lsp-macos-aarch64`,
or `eudkit-lsp-macos-x86_64` instead of the Windows `.exe` shown here:

```text
Downloading required checksum https://github.com/Tb1281/eudplib/releases/download/v0.1.1/eudkit-lsp-windows-x86_64.exe.sha256
Downloading https://github.com/Tb1281/eudplib/releases/download/v0.1.1/eudkit-lsp-windows-x86_64.exe
Verified SHA-256 checksum for eudkit-lsp-windows-x86_64.exe.
Installed eudkit-lsp 0.1.1 at ...\globalStorage\eudkit.eudkit-vscode\eudkit-lsp\0.1.1\eudkit-lsp-windows-x86_64.exe
Starting eudkit-lsp from GitHub release download: ...
Language server started.
```

The cached directory should contain the executable and its `.sha256`
companion. Delete it, reload the window, and confirm the completion still works
after a fresh download.

## 5. Confirm eudkit diagnostics are visible

If the folder has an `eudkit.toml` and a map with named locations, open a Python
file that uses an unknown Map Symbol. A warning with source `eudkit` should
appear in **Problems**. Confirm Pylance/tsserver type diagnostics are still
present and separate, and that no eudkit type errors are invented.

## 6. Optional: verify the downloaded server directly

This step needs Python but not Rust. Take the cached `eudkit-lsp` path from the
output channel and run the repository smoke script. Windows uses `python` where
Linux/macOS use `python3`:

```sh
python3 scripts/smoke_eudkit_lsp.py \
  /path/to/globalStorage/eudkit.eudkit-vscode/eudkit-lsp/0.1.1/eudkit-lsp-linux-x86_64 \
  --expected-version 0.1.1
```

It checks `eudkit-lsp --version`, then a real stdio LSP completion returning all
228 canonical units for both the bare invocation and the `--stdio` invocation
that VS Code uses. Run the equivalent command with `python` and the cached
`.exe` path on Windows.

## 7. Zed dev extension (Rust required for the dev build)

The Zed artifact is a source archive. Zed compiles dev extensions to WebAssembly
locally, so installing it as a dev extension **does** require a Rust toolchain,
even though `eudkit-lsp` itself is downloaded as a native binary. This is a
limitation of dev-extension installation, not of the language server; a future
Zed registry package removes that step.

On a machine with Rust:

1. Extract `eudkit-zed-0.1.1.tar.gz`.
2. In Zed: `zed: extensions` → **Install Dev Extension** → select the extracted
   `eudkit-zed-0.1.1` directory.
3. Make sure `EUDKIT_LSP_PATH` is unset and `eudkit-lsp` is not on `PATH`.
4. Open a Python file and type `KillUnit("`.
5. Check `zed: open log` for the pinned `v0.1.1` download, `verified SHA-256
   checksum`, and `installed` lines.

The Zed extension cannot execute `eudkit-lsp --version` through
`zed_extension_api` 0.7. Its version compatibility is enforced by pinning the
download URL and cache to `CARGO_PKG_VERSION` (`0.1.1` → tag `v0.1.1`) and by
requiring the published checksum. See
[`editors/zed/README.md`](../editors/zed/README.md) and
[`editors/zed/TESTING.md`](../editors/zed/TESTING.md).

## 8. Failure-path spot checks

- **Offline download**: disable the network, remove the cache, reload VS Code.
  The extension must show an actionable download error and offer the path/env
  overrides; it must not start an unverified binary.
- **Missing checksum**: temporarily remove a `.sha256` asset from a test
  release. Installation must fail with `required .sha256 companion` and leave
  no cached executable.
- **Checksum mismatch**: publish a wrong checksum in a test release. The
  extension must report both hashes and delete the download.
- **Version mismatch**: point `eudkit.lsp.path` at a server from another
  major/minor series. The extension must reject it with the `0.1.x` rule.
- **Unsupported platform**: on a platform with no asset, the extension must
  name the supported platforms and list the local override options.

Record the platform, OS version, VS Code version, Zed version (if run), release
tag, result, and any output channel excerpts when running this for a release.
