# Testing the VS Code extension

VS Code is not installed in this repository's Linux test environment, so this
document defines the Extension Development Host smoke test instead of an
automated VS Code integration test. The Rust language server itself has
automated protocol tests in `src/rust/eudkit-lsp/tests/protocol.rs`, including
the assertion that completion returns exactly 228 canonical unit names.

## 1. Build the language server

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

The debug binary is now at `../../src/rust/target/debug/eudkit-lsp`
(`eudkit-lsp.exe` on Windows). Confirm it exists and print its absolute path:

```bash
BIN="$(cd ../../src/rust && pwd)/target/debug/eudkit-lsp"
ls -l "$BIN"
echo "$BIN"
```

On Windows PowerShell, from `editors/vscode`:

```powershell
$Bin = (Resolve-Path ..\..\src\rust\target\debug\eudkit-lsp.exe).Path
Test-Path $Bin
$Bin
```

## 2. Build and check the extension

From `editors/vscode`:

```bash
npm install
npm run check
npm test
npm run build
```

## 3. Start the Extension Development Host

1. Open the `editors/vscode` folder in VS Code. From `editors/vscode`:

   ```bash
   code .
   ```

2. Set `eudkit.lsp.path` to the absolute binary path printed above. Open
   **Preferences: Open User Settings (JSON)** and add:

   ```json
   "eudkit.lsp.path": "/absolute/path/to/eudplib/src/rust/target/debug/eudkit-lsp"
   ```

   This is a user setting so the machine-specific path is not written into the
   repository.

3. Press <kbd>F5</kbd> and choose **Run eudkit Extension** if prompted. The
   committed `.vscode/launch.json` starts an Extension Development Host with
   the extension under development and opens `test/fixtures` as its workspace.

## 4. Confirm canonical unit completion

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

The `eudkit LSP` output channel should contain lines showing which source was
used, the server path, and the `running` lifecycle state.

## 5. Check restart and crash handling

- Run **eudkit: Restart Language Server** from the Command Palette. The output
  channel should show a stop/start pair.
- Change `eudkit.lsp.path` to a different path (or an invalid path). The
  extension restarts automatically. An invalid path produces an actionable
  error listing the paths that were tried.
- To exercise the crash path, set a valid path, start the server, then kill the
  `eudkit-lsp` process from a terminal. The extension reports that the server
  stopped unexpectedly and offers **Restart** and **Show Output** actions.

## 6. Package the extension

From `editors/vscode`:

```bash
npm run package
```

The VSIX is written to `dist/eudkit-vscode-0.1.0.vsix`. It contains the bundled
`dist/extension.js`, `package.json`, `README.md`, `TESTING.md`, and `LICENSE`;
it does not contain `node_modules`.

Inspect the archive if needed:

```bash
unzip -l dist/eudkit-vscode-0.1.0.vsix
```
