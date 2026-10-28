# Zed smoke test: `eudkit-lsp` feature parity

This walks through a dev-extension install and verifies the things that matter:

1. Zed starts the shared `eudkit-lsp` for Python, TypeScript, TSX, and
   JavaScript.
2. `KillUnit("` shows exactly the 228 canonical unit names in every language.
3. Map Context locations and switches come from `eudkit.toml`, with the
   canonical fallback when no project file is present.
4. Unknown Map Symbol and EUD player warnings match the shared server, without
   Stock Unit Filter false positives.

It also covers the release-download path, version pinning, cache layout, and
mandatory checksum verification. Terms such as Map Context, Map Symbol, and
Stock Unit Filter are defined in [`../../GLOSSARY.md`](../../GLOSSARY.md).

## Prerequisites

- Zed with extension support. Current Zed builds dev extensions for
  `wasm32-wasip2`; the manual build check below also uses `wasm32-wasip1`.
- Rust installed through `rustup` for the dev-extension WASM build. The
  downloaded `eudkit-lsp` binary itself does not need Rust.
- Run the commands from the repository root unless a step says otherwise.

```sh
rustup target add wasm32-wasip2 wasm32-wasip1
```

## Parity checklist

Use this as the acceptance summary; the numbered sections hold the detailed
steps.

- [ ] Python: canonical `KillUnit("` completion, Map Context completion,
      unknown-symbol warnings, and EUD player warnings.
- [ ] TypeScript: the same behavior in `.ts` files.
- [ ] TSX: the same behavior in `.tsx` files (the extension reports
      `typescriptreact`).
- [ ] JavaScript: the same behavior in `.js` files. `.jsx` files open as Zed's
      JavaScript language.
- [ ] With `eudkit.toml`: `Bring(..., "` includes map locations and
      `SetSwitch("` includes named map switches.
- [ ] Without `eudkit.toml`: locations fall back to `Anywhere`, switches to
      `Switch 1`..`Switch 256`, and `KillUnit("` stays canonical.
- [ ] Unknown Map Symbols warn with source `eudkit`; Stock Unit Filters,
      numeric stock IDs, `UNIx` names, and free-form strings do not.
- [ ] EUD group-player warnings appear for offsets above 232 and stay silent
      for stock units, concrete players, variables, and ambiguous expressions.
- [ ] Warnings clear when the document is fixed or closed.
- [ ] No `eudkit.inputMap` or map-path setting is introduced.

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
# eudkit-lsp 0.1.2
```

Verify the LSP itself returns all 228 units before involving Zed. The
repository script works on Linux, macOS, and Windows:

```sh
python scripts/smoke_eudkit_lsp.py \
  src/rust/target/release/eudkit-lsp --expected-version 0.1.2
```

The last lines should read:

```text
... reports eudkit-lsp 0.1.2
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
`eudkit-zed-0.1.2.tar.gz` from a GitHub Release. Extract it and select the
extracted `eudkit-zed-0.1.2` directory. It contains `extension.toml`,
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

## 4. Completion smoke test: Python

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

## 5. Completion smoke test: TypeScript, TSX, and JavaScript

The shared server supplies identical behavior; only the Zed language
registration differs. Use scratch files outside the repository, for example
`/tmp/eudkit-zed-parity/kill_unit.ts`, `.tsx`, and `.js`:

```ts
import { KillUnit, Player1 } from "eudkit";

KillUnit("", Player1);
```

```tsx
import { KillUnit, Player1 } from "eudkit";

const view = <div />;
KillUnit("", Player1);
```

```js
import { KillUnit, Player1 } from "eudkit";

KillUnit("", Player1);
```

1. Open each file. Replace the empty string and closing quote so the line ends
   with `KillUnit("`, then place the cursor after the quote.
2. Trigger completion if needed and confirm the same 228 canonical names.
3. Confirm stock filters and `UNIx` names are absent, and that completion is
   absent in `DisplayText("`, in the player argument, in comments, and in
   ordinary strings.
4. Confirm tsserver still supplies hovers and type diagnostics. tsserver may
   report a missing `eudkit` module in a scratch folder; that warning is
   separate from eudkit diagnostics.

Zed reports `.ts` as `TypeScript`, `.tsx` as `TSX`, and `.js`/`.jsx` as
`JavaScript`. The extension maps TSX to `typescriptreact`. Zed has no separate
JSX language, so `.jsx` files report the JavaScript language id.

## 6. Map Context parity with and without `eudkit.toml`

`eudkit-lsp` searches from the document directory upward for the nearest
`eudkit.toml`. Create a disposable project from the repository's v2 fixture
map:

```sh
mkdir -p /tmp/eudkit-zed-parity
cp test_maps/v2/verify_v2.scx /tmp/eudkit-zed-parity/map.scx
cat > /tmp/eudkit-zed-parity/eudkit.toml <<'EOF'
[project]
name = "zed-parity"
input_map = "map.scx"
output_map = "out.scx"
EOF
```

Open the scratch Python, TypeScript, TSX, and JavaScript files from
`/tmp/eudkit-zed-parity`.

### With `eudkit.toml`

1. Type `Bring(AllPlayers, AtLeast, 1, "Terran Marine", "` and trigger
   completion. The list should contain `Anywhere` and
   `=== [EUDKIT V2 TEST MAP] ===` from the map.
2. Type `SetSwitch("`. The list should contain `Switch 1`..`Switch 256` plus
   any switch named in the map's `SWNM` section, with each name appearing once.
3. Type `KillUnit("`. It should still contain exactly the 228 canonical unit
   names. Map-defined custom units are accepted as symbols but are not added to
   this list.
4. Repeat in all four languages. The labels must be identical.

### Without `eudkit.toml`

1. Rename or move the project file so the document no longer has an
   `eudkit.toml` above it:

   ```sh
   mv /tmp/eudkit-zed-parity/eudkit.toml /tmp/eudkit-zed-parity/eudkit.toml.off
   ```

2. `Bring(..., "` should offer exactly `Anywhere`.
3. `SetSwitch("` should offer exactly `Switch 1`..`Switch 256`.
4. `KillUnit("` should still offer the 228 canonical units.
5. No eudkit warning should appear for any canonical symbol. Restore the file
   afterward.

## 7. Diagnostics parity

Keep the scratch project with `eudkit.toml` for the unknown-symbol checks.
Open one document per language. Pylance/tsserver may still report their own
type diagnostics; eudkit warnings are the ones with source `eudkit`.

### Unknown Map Symbols

Python:

```python
from eudplib import *

KillUnit("Not A Unit", player)
Bring(AllPlayers, AtLeast, 1, "Terran Marine", "Not A Location")
SetSwitch("Not A Switch")
```

TypeScript or JavaScript:

```ts
import { AllPlayers, AtLeast, Bring, KillUnit, SetSwitch } from "eudkit";

KillUnit("Not A Unit", 1);
Bring(AllPlayers, AtLeast, 1, "Terran Marine", "Not A Location");
SetSwitch("Not A Switch");
```

Expect three warnings with source `eudkit`: `Unknown unit`, `Unknown location`,
and `Unknown switch`. Fix one line and confirm its warning clears; close the
document and confirm all eudkit warnings clear.

### No Stock Unit Filter false positives

The same document should produce no eudkit diagnostics for:

```ts
KillUnit("Any unit");
KillUnit("Men");
KillUnit("Buildings");
KillUnit("Factories");
KillUnit(229);
KillUnit("300");
DisplayText("Not A Unit");
MinimapPing("Anywhere");
SetSwitch("Switch 256");
```

Use the equivalent Python call syntax without semicolons.

### EUD player warning

This warning appears only when a group player is combined with a clear EUD
offset above the stock boundary (232). Positive cases:

```python
Deaths(Foes, Exactly, 15, 300)
DeathsX(AllPlayers, Exactly, 15, 300)
SetDeaths(c.Foes, Exactly, 15, 0x12C)
SetDeathsX(eud.AllPlayers, Exactly, 15, +300)
```

TypeScript and JavaScript use the same calls with semicolons. Each should warn
on the player argument, and the message should name the selector and explain
that the unit value is ambiguous between a unit ID and an EUD offset.

These cases must stay silent:

```python
Deaths(Foes, Exactly, 15, "Any unit")
Deaths(Foes, Exactly, 15, 229)
Deaths(Foes, Exactly, 15, 0)
Deaths(Foes, Exactly, 15, 232)
Deaths(Player1, Exactly, 15, 300)
Deaths(CurrentPlayer, Exactly, 15, 300)
Deaths(player, Exactly, 15, 300)
Deaths(Foe, Exactly, 15, 300)
Deaths(Foes + 1, Exactly, 15, 300)
Deaths(Foes, Exactly, 15, 232 + 1)
```

Reintroducing `Deaths(Foes, Exactly, 15, 300)` should bring the warning back.
Close the document and confirm it clears. Run the same checks in TypeScript,
TSX, and JavaScript.

## 8. Server logs

Run Zed with `--foreground` to see extension `stdout` and LSP lifecycle logs,
or use `zed: open log` after the fact. Look for:

- `eudkit-lsp: using ...` or `eudkit-lsp: downloading ...`
- `eudkit-lsp: verified SHA-256 checksum for ...`
- `eudkit-lsp: installed ...`
- A language server start for `eudkit-lsp` when a Python, TypeScript, TSX, or
  JavaScript buffer opens
- No `failed to download`, `checksum`, or `unsupported platform` errors

If the binary cannot be started, Zed marks the language server as failed and
the extension returns an error naming the asset and checksum URLs and the
`EUDKIT_LSP_PATH` / `PATH` overrides.

## 9. Release-download path and mandatory checksum

Use this after a release containing the `eudkit-lsp-*` assets exists. The
extension package version is `0.1.2`, so the download is pinned to tag
`v0.1.2` and the cache directory `eudkit-lsp-0.1.2/`.

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

3. Restart Zed with `--foreground` and open a Python, TypeScript, TSX, or
   JavaScript file. The log should show a download from:

   ```text
   https://github.com/Tb1281/eudplib/releases/download/v0.1.2/<asset>
   ```

   followed by `verified SHA-256 checksum` and `installed`. The version in the
   URL comes from `CARGO_PKG_VERSION`; it is not read from the downloaded
   binary because zed_extension_api 0.7 cannot execute arbitrary commands.

4. Inspect the cache. It contains the platform asset plus its `.sha256`
   companion, for example on Linux:

   ```text
   ~/.local/share/zed/extensions/work/eudkit-lsp/eudkit-lsp-0.1.2/eudkit-lsp-linux-x86_64
   ~/.local/share/zed/extensions/work/eudkit-lsp/eudkit-lsp-0.1.2/eudkit-lsp-linux-x86_64.sha256
   ```

5. Verify the checksum manually:

   ```sh
   cd ~/.local/share/zed/extensions/work/eudkit-lsp/eudkit-lsp-0.1.2
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

## 10. No map-path or inputMap setting

The extension intentionally has no map-path or `eudkit.inputMap` setting.
`eudkit-lsp` discovers `eudkit.toml` from the document path. Confirm with:

```sh
rg -n "map[_-]?path|mapPath" editors/zed/src editors/zed/extension.toml
rg -n "inputMap|input_map" editors/zed/extension.toml
cargo test manifest_defines_no_input_map_setting
```

There should be no matches in `extension.toml`, and the host regression test
must pass. The Zed extension must not invent a Zed-only setting.

## Troubleshooting

- **Completion list is empty**: confirm the buffer language is Python,
  TypeScript, TSX, or JavaScript, the extension is installed, and `KillUnit("`
  is the first argument. Check `zed: open log` for the language server start
  line.
- **Map Context completion is empty**: confirm an `eudkit.toml` with a readable
  `input_map` is at or above the document, and that the document is not inside
  a nested project with a different `eudkit.toml`.
- **`.jsx` behaves differently from `.tsx`**: Zed has no separate JSX language;
  `.jsx` files use the JavaScript language id. The shared server's JSX handling
  is selected only when a client reports `javascriptreact`.
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
