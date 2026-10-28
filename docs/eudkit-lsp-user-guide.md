# eudkit LSP user guide (MVP)

This guide describes the reference user experience for the eudkit language
server in the `0.1.x` editor MVP: the behavior the release is accepted against.
It covers VS Code and Zed, Python/TypeScript/JavaScript, projects with and
without `eudkit.toml`, the warnings the server publishes, hover and signature
help, and what is deliberately not included yet.

Hover and signature help were added on the development branch after `v0.1.1`.
They are **not** part of the published `v0.1.1` VSIX, Zed archive, or
`eudkit-lsp` binaries; the next release will ship them. The rest of this guide
describes the current development tree.

Terms such as **Map Context**, **Map Symbol**, **Canonical Symbol**, and
**Stock Unit Filter** are defined in [`GLOSSARY.md`](../GLOSSARY.md).

## What the MVP provides

- Completion for the 228 canonical StarCraft unit names in the unit argument of
  `KillUnit(...)` and other supported unit arguments, for example after
  `KillUnit("`.
- Completion for Map Context locations (for example the location argument of
  `Bring`) and switches (for example `SetSwitch("`) when an `eudkit.toml`
  project with a readable input map is found.
- A canonical fallback when there is no project: `Anywhere`, `Switch 1` through
  `Switch 256`, and the 228 unit names.
- Unknown Map Symbol warnings and EUD group-player warnings as advisory LSP
  diagnostics with source `eudkit`.
- Hover for canonical and Map Context unit, location, and switch strings. The
  hover names the symbol, says whether it is canonical or map-defined, and
  shows its ID. Stock Unit Filters show their filter ID.
- Hover for the supported eudkit calls listed below, showing the call
  signature, a one-line summary, the return type, and which parameters accept
  symbols.
- Signature help for those calls in Python, TypeScript, and JavaScript. The
  active parameter follows commas and Python keyword arguments, and qualified
  TypeScript/JavaScript calls such as `eudkit.KillUnit(...)` are supported.
- Zero map configuration. Neither extension has a map-path setting and neither
  asks for a map path.

The extensions contain no completion, hover, signature, or diagnostic logic of
their own. VS Code and Zed both start the same `eudkit-lsp` binary, so they
return the same symbols, the same warnings, the same hover text, and the same
signatures.

The LSP does **not** provide general API types, per-map stubs, or type checking.
Hover and signature help are intentionally narrow: they resolve canonical and
Map Context symbols and the supported eudkit calls listed below, not arbitrary
expressions or the full SDK API. Pylance (Python) and tsserver
(TypeScript/JavaScript) remain the source of general type information and type
diagnostics and run alongside eudkit.

## Requirements

| Component | Requirement |
| --- | --- |
| VS Code | 1.91 or newer. |
| Zed | A Zed build with extension support. The extension was loaded and started with Zed 1.25.3; see the [acceptance matrix](eudkit-lsp-acceptance-matrix.md) for the verified scope. |
| Platform | Windows x64, Linux x64, macOS arm64, or macOS x86_64. |
| Rust | Not needed for VS Code. Not needed for `eudkit-lsp` in either editor. Needed only to compile the Zed **dev extension** to WebAssembly. |
| Map | Optional. Without `eudkit.toml` the server uses Canonical Symbols only. |

The VSIX is platform-neutral. The native `eudkit-lsp` server is downloaded for
the platform running the editor. Other platform/architecture pairs can use a
locally built server through the override described below.

## Install

### VS Code (VSIX, no Rust)

1. From the matching GitHub Release, download:

   - `eudkit-vscode-<version>.vsix`
   - `eudkit-vscode-<version>.vsix.sha256`

2. Verify the checksum. Linux/macOS:

   ```sh
   sha256sum -c eudkit-vscode-<version>.vsix.sha256
   ```

   Windows PowerShell:

   ```powershell
   Get-FileHash .\eudkit-vscode-<version>.vsix -Algorithm SHA256
   Get-Content .\eudkit-vscode-<version>.vsix.sha256
   ```

   The two hashes must match.

3. Install the VSIX:

   ```sh
   code --install-extension ./eudkit-vscode-<version>.vsix
   ```

   On Windows PowerShell, use `.\eudkit-vscode-<version>.vsix`.

4. Reload the VS Code window. On first use the extension resolves
   `eudkit-lsp`, downloads the matching release asset when it cannot find a
   local server, verifies its mandatory SHA-256 companion, runs
   `eudkit-lsp --version`, and starts the client.

The normal clean-install path needs no Rust toolchain. The **eudkit LSP**
output channel records every source, URL, checksum, version probe, and error.

### Zed (dev extension)

Zed does not yet have a registry package for this extension. Install the
published source archive as a dev extension:

1. From the matching GitHub Release, download:

   - `eudkit-zed-<version>.tar.gz`
   - `eudkit-zed-<version>.tar.gz.sha256`

2. Verify the archive checksum with `sha256sum -c` or `shasum -a 256 -c`.

3. Install the WebAssembly target used by current Zed dev extensions:

   ```sh
   rustup target add wasm32-wasip2
   ```

   The published extension README also lists `wasm32-wasip1` for older Zed
   builds.

4. Extract the archive.

5. In Zed, run `zed: extensions`, click **Install Dev Extension**, and select
   the extracted directory containing `extension.toml`.

6. Open a Python, TypeScript, TSX, or JavaScript file. The extension downloads
   the native `eudkit-lsp` asset, verifies its SHA-256 companion, and starts it.
   The server itself does not need Rust.

The Rust requirement is for compiling the dev extension to WebAssembly, not for
running `eudkit-lsp`. A future registry package removes that dev-extension
build step.

### Local server override

Use an override for development, unsupported platforms, or offline use.

VS Code, in order of precedence:

1. The `eudkit.lsp.path` setting. Absolute, or relative to the first workspace
   folder.
2. The `EUDKIT_LSP_PATH` environment variable.
3. `eudkit-lsp` on `PATH`.
4. `bin/<platform>-<arch>/eudkit-lsp` under the extension root.
5. A checksum-verified GitHub Release download pinned to the extension version.

Zed, in order of precedence:

1. `EUDKIT_LSP_PATH` from the environment Zed was launched with.
2. `eudkit-lsp` on `PATH`.
3. The version-scoped download cache.
4. A fresh GitHub Release download followed by mandatory checksum
   verification.

`EUDKIT_LSP_PATH` points at the **language server**, not at the `eudkit` build
CLI. The two binaries are separate release assets.

## First run

### VS Code

1. Create an empty folder, or use an existing project.
2. Add a Python, TypeScript, or JavaScript file.
3. Open the **eudkit LSP** output channel (Command Palette → **Output: Show
   Output Channel** → `eudkit LSP`). With a clean install you should see the
   download, checksum verification, version check, and `Language server
   started.` lines.
4. Type `KillUnit("` and trigger completion if it does not open automatically
   (<kbd>Ctrl</kbd>+<kbd>Space</kbd>).
5. Confirm the list contains `Terran Marine`, `Zerg Zergling`, and
   `Protoss Zealot`, and does not contain Stock Unit Filters or custom `UNIx`
   names.

If you need to restart the server, run **eudkit: Restart Language Server** from
the Command Palette.

### Zed

1. Open a project and a Python, TypeScript, TSX, or JavaScript file.
2. Type `KillUnit("` and run `editor: completions` if the menu is not already
   open.
3. Confirm the same 228 canonical unit names.
4. Use `zed: open log` to inspect extension and language-server startup
   messages when something fails.

### Minimal examples

Python:

```python
from eudplib import KillUnit, Player1

KillUnit("
```

TypeScript:

```ts
import { KillUnit, Player1 } from "eudkit";

KillUnit("
```

JavaScript:

```js
import { KillUnit, Player1 } from "eudkit";

KillUnit("
```

Completion is available only inside supported eudkit symbol arguments: the unit
argument of `KillUnit`, `KillUnitAt`, `Deaths`, and related functions; the
location arguments of `Bring`, `KillUnitAt`, `MinimapPing`, and related
functions; and the switch argument of `Switch` and `SetSwitch`. It does not
appear in free-form strings such as `DisplayText("`, in player arguments, in
comments, or inside arbitrary strings. In TypeScript and JavaScript, only plain
string literals are handled; template literals and other expressions are
ignored.

A missing `eudkit` module warning from tsserver, or a missing `eudplib`
import warning from Pylance, is separate from eudkit. Install the SDK package
(or configure `tsconfig`/`jsconfig` paths) for types. eudkit completion still
works without the package installed.

## Hover and signature help

Hover and signature help are available in Python, TypeScript, TSX, JavaScript,
and JSX. They use the same shared server, so both editors show the same text.
This support is on the development branch after `v0.1.1`; none of the released
`v0.1.1` assets contain it.

### Symbol hover

Hover over a quoted symbol argument to see:

- the symbol name;
- whether it is a **Canonical** symbol or a **Map Context** symbol (a name
  from the loaded map);
- its numeric ID; and
- for a map-defined alias of a canonical unit, the canonical name.

A Stock Unit Filter such as `Any unit`, `Men`, `Buildings`, or `Factories`
shows `Stock Unit Filter` and its filter ID. Numeric IDs and unknown names show
no hover, matching the warning validator: numeric values are valid but are not
resolved to a name.

```python
KillUnit("Terran Marine", Player1)          # Canonical unit · ID 0
Bring(AllPlayers, AtLeast, 1, "Terran Marine", "Anywhere")
                                             # Canonical location · ID 64
SetSwitch("Switch 1")                        # Canonical switch · ID 0
```

With an `eudkit.toml` project, a custom `UNIx` unit name or a named location
shows `Map Context unit` or `Map Context location`; a named location also
shows its map bounds. Custom `UNIx` names remain absent from completion but are
recognized by hover when the map is loaded.

### Call hover and signature help

Hover over one of the supported call names to see its signature, a one-line
summary, its return type, and which parameters accept symbols:

- `KillUnit`
- `Deaths`, `DeathsX`, `SetDeaths`, `SetDeathsX`
- `Bring`
- `CreateUnit`, `KillUnitAt`, `RemoveUnitAt`
- `MinimapPing`
- `MoveLocation`, `MoveUnit`
- `GiveUnits`, `Order`
- `Switch`, `SetSwitch`

Signature help opens after `(` and `,`. The active parameter follows commas.
In Python, a keyword argument such as `KillUnit(unit=...)` selects the matching
parameter even when it is not in its positional slot. TypeScript and JavaScript
support both the imported form (`KillUnit(...)`) and the qualified namespace
form (`eudkit.KillUnit(...)`).

```python
KillUnit(
#        ^ signature help: unit, then player after the comma
KillUnit(unit="
#        ^ active parameter: unit
```

The server shows documentation only for these calls. It does not type-check
arguments, infer variable types, or provide hovers for arbitrary expressions,
variables, imports, or SDK APIs outside this list. Pylance and tsserver remain
responsible for general type information. A qualified call from an unrelated
object only produces completion, hover, or signature help when its member name
is one of the supported eudkit calls above.

## Project mode: `eudkit.toml` and Map Context

The server searches from the document's directory upward for the nearest
`eudkit.toml`. That file's `input_map` is loaded into a read-only Map Context.
No map path is configured in the editor.

```toml
[project]
name = "my-map"
input_map = "maps/base.scx"
output_map = "out.scx"
```

- `input_map` may be absolute, or relative to the directory containing
  `eudkit.toml`.
- The map must exist and be readable by the server.
- `output_map` is not used by the LSP.
- If the nearest project has no `input_map`, the server silently uses Canonical
  Symbols. If the file cannot be parsed or the map cannot be loaded, the server
  logs the error to stderr and falls back to Canonical Symbols. Neither case
  fails the editor session.
- The nearest `eudkit.toml` wins for each document.
- The Map Context is reloaded when the project file or the input map changes.
- Nested projects can use different maps; each document uses its own nearest
  project.

### With and without a Map Context

| Behavior | With `eudkit.toml` + readable map | Without a usable project |
| --- | --- | --- |
| `KillUnit("` completion | 228 canonical units | 228 canonical units |
| Custom `UNIx` names in completion | Not added | Not added |
| Location completion | `Anywhere` + named map locations | `Anywhere` only |
| Switch completion | Named map switches + `Switch 1`..`Switch 256` | `Switch 1`..`Switch 256` |
| Unknown-symbol warnings | Use Map Symbols and canonical names | Use canonical names only |
| Custom unit names in diagnostics | Accepted when present in the map | Warn as unknown |
| `Location N` diagnostics | `Anywhere`, `Location 1`..`Location 255` except `Location 64`; named map locations | `Anywhere`, `Location 1`..`Location 255` except `Location 64` |
| Numeric IDs/offsets | Accepted | Accepted |

Location completion does not list every `Location 1`..`Location 255`; those
canonical names are still accepted if typed. `Location 64` is the canonical
`Anywhere` location; its canonical name is `Anywhere`, not `Location 64`.
Switch completion always includes the 256 canonical switch names and adds
named map switches without duplicates.

### Custom `UNIx` names

Custom unit names from the map's unit settings are recognized by the warning
validator when a Map Context is loaded. They are deliberately **not** added to
`KillUnit` completion. Completion stays exactly the 228 canonical names, so the
list does not change size or order between maps.

### Stock Unit Filters

`Any unit`, `Men`, `Buildings`, and `Factories` are valid unit-list values in
stock triggers. They are accepted by diagnostics and are not suggested by
completion. Numeric unit IDs and numeric EUD offsets are also accepted.

## Warnings

Warnings are published with source `eudkit` and severity **Warning**. They are
advisory: they never block `eudkit build` and never replace Pylance or tsserver
diagnostics.

### Unknown Map Symbols

The server checks closed string literals that occupy a symbol-typed argument:

| Slot | Examples | Accepted values |
| --- | --- | --- |
| Unit | first argument of `KillUnit`, unit argument of `KillUnitAt` | canonical units, map custom units, numeric IDs, Stock Unit Filters |
| Location | location arguments of `Bring`, `KillUnitAt`, and related functions | `Anywhere`, `Location 1`..`Location 255` except `Location 64`, named map locations |
| Switch | `SetSwitch` | `Switch 1`..`Switch 256`, named map switches |

Examples:

```python
KillUnit("Terran Marine")          # accepted
KillUnit("Any unit")               # accepted Stock Unit Filter
KillUnit("300")                    # accepted numeric ID/offset
KillUnit("Not A Unit")             # warning

Bring(AllPlayers, AtLeast, 1, "Terran Marine", "Anywhere")   # accepted
Bring(AllPlayers, AtLeast, 1, "Terran Marine", "Not A Location")  # warning

SetSwitch("Switch 1")              # accepted
SetSwitch("Not A Switch")          # warning
```

Dynamic values, f-strings, template literals, variables, and unclosed strings
being typed are not diagnosed. Closing the document clears its warnings.

### EUD group-player warning

`Deaths`, `DeathsX`, `SetDeaths`, and `SetDeathsX` reuse the unit field as a
memory offset when the player selector is a concrete player. A group player
with an explicit unit number above the stock boundary (232) is ambiguous: the
number may be a unit ID or an EUD offset.

```python
SetDeaths(AllPlayers, SetTo, 1, 300)   # warning
SetDeaths(Player1, SetTo, 1, 300)      # accepted: concrete player
SetDeaths(AllPlayers, SetTo, 1, 200)   # accepted: stock unit range
SetDeaths(AllPlayers, SetTo, 1, "Terran Marine")  # accepted
```

The warning is attached to the player argument and tells the author to use a
concrete player such as `Player1` for EUD triggers. Variables, arithmetic, and
other ambiguous expressions stay silent.

## Troubleshooting

### No completion appears

- Confirm the editor's language mode is Python, TypeScript, or JavaScript
  (TSX/JSX are also registered).
- Put the cursor inside a supported symbol argument, for example after the
  opening quote in `KillUnit("`, `Bring(..., "` for a location, or
  `SetSwitch("`.
- Trigger completion manually. In VS Code use <kbd>Ctrl</kbd>+<kbd>Space</kbd>;
  in Zed run `editor: completions`.
- Check the VS Code **eudkit LSP** output channel, or `zed: open log`. If the
  server is still downloading or failed to start, the editor may show only its
  own word-based suggestions.
- Restart the server with **eudkit: Restart Language Server** in VS Code.

### Map locations or switches are missing

- Confirm an `eudkit.toml` exists at or above the document. A file in a sibling
  folder is not used.
- Confirm `input_map` is non-empty, relative paths are correct, and the map is
  readable.
- Confirm the nearest project is the one you expect; nested projects override
  outer ones for documents below them.
- Check the server stderr in the editor logs. Invalid TOML and unreadable maps
  are logged. A missing or empty `input_map` falls back silently.
- Completion offers named locations plus `Anywhere`. Canonical names
  `Location 1` through `Location 255` (except `Location 64`) are valid if typed
  but are not listed. `Location 64` is `Anywhere`.

### Pylance or tsserver shows errors or missing imports

eudkit does not synthesize type errors or general type information. It
publishes Map Symbol and EUD warnings with source `eudkit`, plus the narrow
hover and signature help described above.

- Install the Python SDK (`eudplib`) or the TypeScript SDK (`eudkit`) for API
  types.
- For a scratch TypeScript project, configure `tsconfig.json` `paths` or
  install the package; otherwise tsserver reports a missing module.
- In the Problems panel, filter by source to separate `eudkit` warnings from
  Pylance/`typescript` diagnostics.
- Both providers can report on the same file. That is expected; eudkit does not
  suppress or replace the language server for the host language.

### No hover or signature help appears

- Hover only resolves canonical or Map Context unit, location, and switch
  strings, plus the supported call names listed in
  [Hover and signature help](#hover-and-signature-help). Variables, numeric
  arguments, dynamic expressions, unsupported APIs, and free-form strings do
  not produce eudkit hover.
- Signature help is available only inside the supported calls, after `(` or
  `,`. A `DisplayText(...)` or other unsupported call returns nothing.
- Confirm the `eudkit-lsp` version is from the development branch or a release
  after `v0.1.1`. The published `v0.1.1` assets do not contain hover or
  signature help.
- Pylance and tsserver may show their own hover and signature results on the
  same symbol. That is expected.

### Download, checksum, or version failure

- A `<asset>.sha256` companion is mandatory. There is no "checksum when
  available" fallback.
- A missing checksum, checksum mismatch, 404, offline network, or version
  mismatch is an actionable error naming the URL and the local override
  options.
- VS Code runs `eudkit-lsp --version` and accepts only the same `0.1.x` series.
  An `0.1.x` extension rejects an `0.2.x` server, and vice versa.
- Zed cannot execute `eudkit-lsp --version` through its extension API 0.7. It
  enforces compatibility structurally: the download URL and cache are pinned to
  the extension package version, and the checksum is mandatory. Do not copy a
  binary from another series into the cache.
- On an unsupported platform, set `eudkit.lsp.path` (VS Code),
  `EUDKIT_LSP_PATH` (either editor), or put a locally built `eudkit-lsp` on
  `PATH`.

### Zed dev extension does not build or start

- Install the Rust toolchain and `wasm32-wasip2` (and `wasm32-wasip1` for older
  Zed builds).
- Check `zed: open log` for the WebAssembly build error and the downloaded
  asset URL.
- Confirm `EUDKIT_LSP_PATH`, if set, points at the `eudkit-lsp` server binary
  and was present in the environment that launched Zed.
- If the extension was installed from a source archive, select the extracted
  directory that directly contains `extension.toml`.

### Custom units are accepted but not suggested

That is intentional. Custom `UNIx` names are map-specific; completion remains
the 228 canonical names so it is stable across maps. Custom names are still
accepted by diagnostics when the map is loaded.

## MVP exclusions

The following are explicitly outside the `0.1.x` editor MVP:

- Per-map `.d.ts`/`.pyi` generation by the editor or language server, and
  automatic wiring of generated stubs into Pylance/tsserver. The `eudkit` CLI's
  `inspect --dts` and build-time declaration generation are separate compiler
  features, not part of this editor acceptance.
- Inline compiler diagnostics. `eudkit build` errors are not published in the
  editor Problems panel; run the CLI or CI for compile errors.
- A map-path setting or map picker. There is no `eudkit.inputMap` setting.
- Custom `UNIx` names in `KillUnit` completion.
- General type information or type checking from eudkit. Hover and signature
  help cover only canonical/Map Context symbols and the supported eudkit calls;
  they are not a substitute for Pylance or tsserver.
- Automatic installation of the Python or TypeScript SDK.
- A Zed registry package. The MVP ships a dev-extension source archive.
- Editor-specific completion, hover, signature, or diagnostic logic. Both
  editors share the server.

Hover and signature help are post-`v0.1.1` development work and are not present
in the published `v0.1.1` artifacts. The release notes stay historical; the next
release will include this support.

## Related documentation

- [Acceptance matrix](eudkit-lsp-acceptance-matrix.md)
- [v0.1.1 release notes](releases/v0.1.1.md)
- [Clean-machine editor install smoke test](editor-install-smoke-test.md)
- [VS Code extension README](../editors/vscode/README.md)
- [VS Code testing](../editors/vscode/TESTING.md)
- [Zed extension README](../editors/zed/README.md)
- [Zed testing](../editors/zed/TESTING.md)
