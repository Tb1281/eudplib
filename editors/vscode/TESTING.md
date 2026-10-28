# Testing the VS Code extension

This document covers three levels:

1. Unit tests and type fixtures that run without VS Code.
2. An Extension Development Host smoke test with a locally built server.
3. The published-artifact path: download, checksum/version verification, and a
   clean-machine install. The no-Rust install check lives in
   [`docs/editor-install-smoke-test.md`](../../docs/editor-install-smoke-test.md).

VS Code is not installed in this repository's Linux test environment, so the
UI checks below are manual. The Rust server has automated protocol tests in
`src/rust/eudkit-lsp/tests/protocol.rs`.

## 1. Unit tests and type fixtures

From `editors/vscode`:

```bash
npm ci
npm test           # release helpers, server-path resolution, tsc/checkJs fixtures
npm run check      # strict TypeScript, no emit
npm run test:types # only the TypeScript and JavaScript fixtures
```

`npm test` covers:

- release asset mapping for Windows x64, Linux x64, macOS arm64, and macOS x86_64;
- unsupported-platform errors;
- release URL/tag and cache-path construction;
- SHA-256 parsing, filename checks, known vectors, and mismatch errors;
- `eudkit-lsp --version` parsing and the `0.1.x` compatibility rule;
- a fake-fetch download that verifies checksum, executable mode, cache contents,
  and version probing;
- cached reuse, missing-checksum failure, checksum mismatch, and incompatible
  downloaded binaries.

## 2. Build the language server for development

If the checkout's submodules are not initialized yet, run this once from the
repository root:

```bash
git submodule update --init --recursive
```

From the repository root:

```bash
cd src/rust
cargo build -p eudkit-lsp
cd ../../editors/vscode
```

The debug binary is at `../../src/rust/target/debug/eudkit-lsp`
(`eudkit-lsp.exe` on Windows). Confirm it answers the new version contract:

```bash
../../src/rust/target/debug/eudkit-lsp --version
# eudkit-lsp 0.1.0
```

You can also run the protocol smoke test against it:

```bash
python3 ../../scripts/smoke_eudkit_lsp.py \
  ../../src/rust/target/debug/eudkit-lsp --expected-version 0.1.0
```

## 3. Start the Extension Development Host

1. Open the `editors/vscode` folder in VS Code:

   ```bash
   code .
   ```

2. Set `eudkit.lsp.path` to the absolute binary path printed above. Open
   **Preferences: Open User Settings (JSON)** and add:

   ```json
   "eudkit.lsp.path": "/absolute/path/to/eudplib/src/rust/target/debug/eudkit-lsp"
   ```

   Use a user setting so the machine-specific path is not written into the
   repository.

3. Press <kbd>F5</kbd> and choose **Run eudkit Extension** if prompted. The
   committed `.vscode/launch.json` starts an Extension Development Host with
   the extension under development and opens `test/fixtures` as its workspace.

## 4. Confirm canonical unit completion in Python

1. In the Extension Development Host, open `kill_unit.py` from the fixture
   folder.
2. Replace `pass` in `place_marine` with the following (leave the quote open):

   ```python
   KillUnit("
   ```

3. Completion opens after typing the opening quote. If it does not, press
   <kbd>Ctrl</kbd>+<kbd>Space</kbd>.
4. Confirm the list contains canonical unit names such as `Terran Marine`,
   `Terran SCV`, `Zerg Zergling`, and `Protoss Zealot`. Filtering by `Marine`
   is a quick check.
5. Confirm completion is absent outside the unit string, for example in
   `KillUnit(`, and that Pylance still supplies hovers and type diagnostics.

## 5. Confirm completion in TypeScript and JavaScript

1. Open `test/fixtures/typescript/kill_unit.ts` or
   `test/fixtures/javascript/kill_unit.js`.
2. Replace a `KillUnit("Terran Marine", ...)` argument with an open
   `KillUnit("` and trigger completion if needed.
3. Confirm the same 228 canonical names appear, and that completion is absent
   in `DisplayText("`, in the player argument, in comments, and in ordinary
   strings.
4. Confirm tsserver still supplies hovers and type diagnostics.

The `eudkit LSP` output channel should show the resolved source, the
`eudkit-lsp <version>` check, the server path, and the `running` lifecycle
state.

## 6. Confirm eudkit warnings are visible

The extension no longer suppresses eudkit diagnostics. With a project that has
an `eudkit.toml` and an unknown Map Symbol, for example a location name that is
not defined in the map:

1. Open the Python or TypeScript document.
2. Confirm a warning with source `eudkit` appears in the Problems panel.
3. Confirm no eudkit type errors appear: Pylance/tsserver are still the only
   sources of type diagnostics.
4. Close the document and confirm the eudkit warning is cleared.

Warnings are advisory; they must not block `eudkit build`.

## 7. Check restart and crash handling

- Run **eudkit: Restart Language Server** from the Command Palette. The output
  channel should show a stop/start pair and a fresh version check.
- Change `eudkit.lsp.path` to a different path (or an invalid path). The
  extension restarts automatically. An invalid path produces an actionable
  error listing the paths, download URL, and checksum URL that were tried.
- To exercise the crash path, set a valid path, start the server, then kill the
  `eudkit-lsp` process from a terminal. The extension reports that the server
  stopped unexpectedly and offers **Restart** and **Show Output** actions.
- To exercise a version mismatch, point `eudkit.lsp.path` at a binary whose
  `--version` reports a different major/minor series. The extension must reject
  it and say that extension `0.1.x` accepts only `eudkit-lsp` `0.1.x`.

## 8. Test the release-download path

Use this after a release containing the `eudkit-lsp-*` assets exists. The
extension version is `0.1.0`, so the tag must be `v0.1.0`.

1. Clear all local overrides:

   ```bash
   unset EUDKIT_LSP_PATH
   which eudkit-lsp   # should print nothing
   ```

   Also empty `eudkit.lsp.path`. On Windows use
   `Remove-Item Env:EUDKIT_LSP_PATH` and `where eudkit-lsp`.

2. Delete the extension's cached download. The exact path is under VS Code's
   global storage for `eudkit.eudkit-vscode`:

   - Linux: `~/.config/Code/User/globalStorage/eudkit.eudkit-vscode/eudkit-lsp/0.1.0/`
   - macOS: `~/Library/Application Support/Code/User/globalStorage/eudkit.eudkit-vscode/eudkit-lsp/0.1.0/`
   - Windows: `%APPDATA%\Code\User\globalStorage\eudkit.eudkit-vscode\eudkit-lsp\0.1.0\`

3. Start the Extension Development Host (or use the installed VSIX) and open a
   Python file. The output channel should show:

   - `Downloading required checksum https://github.com/Tb1281/eudplib/releases/download/v0.1.0/<asset>.sha256`
   - `Downloading https://github.com/Tb1281/eudplib/releases/download/v0.1.0/<asset>`
   - `Verified SHA-256 checksum`
   - `Installed eudkit-lsp 0.1.0`
   - `Starting eudkit-lsp from GitHub release download`

4. Inspect the cache. It contains the platform asset and its checksum:

   ```text
   eudkit-lsp/0.1.0/eudkit-lsp-linux-x86_64
   eudkit-lsp/0.1.0/eudkit-lsp-linux-x86_64.sha256
   ```

5. Verify the checksum manually:

   ```bash
   cd ~/.config/Code/User/globalStorage/eudkit.eudkit-vscode/eudkit-lsp/0.1.0
   sha256sum -c eudkit-lsp-linux-x86_64.sha256
   ```

   On macOS use `shasum -a 256 -c ...`.

6. Restart the extension. The second start should reuse the cache without a
   network request and still run `--version`.

7. Negative checks: delete the checksum from a temporary release, or serve a
   corrupted asset, and confirm the extension refuses to install it. A missing
   `.sha256` is a hard failure; a hash mismatch reports both hashes and removes
   the downloaded file.

## 9. Package and inspect the VSIX

From `editors/vscode`:

```bash
npm run package
npm run verify:vsix
```

The VSIX is written to `dist/eudkit-vscode-0.1.0.vsix`. It contains:

```text
extension/package.json
extension/readme.md
extension/TESTING.md
extension/LICENSE.txt
extension/dist/extension.js
```

It does not contain `node_modules/`, `src/`, `test/`, `scripts/`, or
`.vscode/`. `package.json` carries the MIT license, the
`https://github.com/Tb1281/eudplib` repository, homepage, and bugs URLs needed
for later Marketplace publication.

Inspect the archive directly if needed:

```bash
unzip -l dist/eudkit-vscode-0.1.0.vsix
```
