# eudkit LSP MVP acceptance matrix

Scope: the `0.1.x` editor MVP described in
[#52](https://github.com/Tb1281/eudplib/issues/52), plus the post-`v0.1.1`
hover/signature work tracked by [#54](https://github.com/Tb1281/eudplib/issues/54).
This is the reference verification record for the user guide at
[`eudkit-lsp-user-guide.md`](eudkit-lsp-user-guide.md).

The released `v0.1.1` assets contain the completion, diagnostics, and Map Context
behavior described in this matrix; they do not contain hover or signature help.
That support was added on the development branch after the release, so rows
marked **post-`v0.1.1`** are verified against the development tree, not the
published `v0.1.1` binaries.

## Environment

| Item | Value |
| --- | --- |
| Date | 2026-10-28 |
| OS | Ubuntu 26.04.1 LTS, x86_64 |
| VS Code | 1.143.1 |
| Zed | 1.25.3 |
| `eudkit-lsp` | 0.1.1 (GitHub Release asset, checksum verified) for release rows; development build for post-`v0.1.1` rows |
| Python | 3.14.4 |
| Rust | 1.99.0 |

The VS Code behavior pass used the real VS Code extension host. The extension
under test was the release `eudkit-vscode-0.1.1.vsix` bundle, checksum verified
and installed in a clean profile; no Rust toolchain was used. The server cache
was cleared, so the first run downloaded `eudkit-lsp-linux-x86_64` and its
`.sha256` companion from the `v0.1.1` release, verified the checksum, installed
it under global storage, probed `--version`, and started it. Subsequent checks
reused the verified cache.

The Zed pass loaded the extension in a dev profile and started the same
`eudkit-lsp` 0.1.1 binary for Python, TypeScript, and JavaScript documents.

## Result summary

| Area | Result | Evidence |
| --- | --- | --- |
| Shared LSP behavior, all languages and contexts | **PASS** | 87 Rust tests, including 34 protocol tests |
| Hover and signature help (post-`v0.1.1` development branch) | **PASS at protocol level** | 8 protocol tests and unit coverage; not present in the published `v0.1.1` assets |
| VS Code reference UX, Python/TypeScript/JavaScript, with and without `eudkit.toml` | **PASS** | all reference checks below |
| VS Code release download, checksum, cache, version probe | **PASS** | Output channel record; cached asset and `.sha256` present |
| Zed extension load and server start, Python/TypeScript/JavaScript | **PASS** | Extension host log: 2 extensions loaded; 3 `eudkit-lsp` process starts |
| Zed completion/diagnostic/hover/signature UI | **NOT EXECUTED** | Zed's stock LSP client was not driven; the extension contains no UI code, and shared server behavior is covered by the protocol matrix |
| No-Rust VSIX installation | **PASS** | Release `eudkit-vscode-0.1.1.vsix` checksum verified and installed in a clean profile with `code --install-extension`; the installed bundle then passed the VS Code behavior matrix; no Rust used. |
| Windows/macOS execution | **Not executed here** | Assets were built and checksum/format verified in the `v0.1.1` release workflow |

### How Zed runs the extension

Zed's language-server extensions have two documented parts
([Developing Extensions](https://zed.dev/docs/extensions/developing-extensions),
[Language Servers](https://zed.dev/docs/extensions/languages#language-servers)):

1. `extension.toml` registers the server under `[language_servers.<id>]` and
   lists the Zed language names it applies to.
2. The Rust `Extension::language_server_command` method returns the executable,
   its arguments, and environment variables.

Zed then starts and supervises that process and renders its LSP responses with
its own client. This extension implements only those two parts: the manifest
registration plus `language_server_command`, which resolves the binary (or
downloads and checksum-verifies it). It returns no arguments and no environment
overrides, and it contains no completion, diagnostic, hover, signature, or UI
code.

The Zed extension API does not expose a test harness equivalent to the VS Code
extension host, and this environment is Wayland-only, so the completion, hover,
and signature UI was not driven programmatically. Because the extension has no
UI code, "the server starts for each MVP language" is the complete
extension-level pass; everything rendered afterwards is Zed's stock LSP client.
This record therefore marks Zed UI rendering as not executed rather than as a
pass or a failure, and does not claim a Zed UI-level completion, diagnostic,
hover, or signature result. The official Zed docs document log-based debugging
(`zed: open log`, `zed --foreground`) and do not describe an automated UI
harness. The manual Zed checklist remains in
[`editors/zed/TESTING.md`](../editors/zed/TESTING.md).

## Shared server protocol matrix

The protocol tests start the real language server over stdio and assert
completion labels, diagnostics, hover text, and signature-help state. All rows
pass for Python, TypeScript, and JavaScript; TypeScript/JSX and TSX registration
are covered by the same suite.

| Case | Python | TypeScript | JavaScript | Protocol tests |
| --- | --- | --- | --- | --- |
| `KillUnit("` returns exactly 228 canonical units | PASS | PASS | PASS | `completes_canonical_units_in_typescript_tsx_javascript_and_jsx`, `initializes_and_completes_canonical_units_for_unterminated_kill_unit_string` |
| Stock Unit Filters are not suggested | PASS | PASS | PASS | `unit_slot_stays_canonical_only_with_a_map_context` and language variants |
| Custom `UNIx` names are not suggested | PASS | PASS | PASS | `typescript_unit_slot_stays_canonical_with_a_map_context` |
| Completion only inside supported symbol arguments, not free-form strings | PASS | PASS | PASS | `does_not_complete_outside_the_kill_unit_unit_slot`, `does_not_complete_outside_the_unit_slot_in_typescript_and_javascript` |
| Qualified TS/JS calls complete only for supported eudkit APIs | n/a | PASS | PASS | `completes_qualified_supported_calls_in_typescript_and_javascript`, `does_not_complete_outside_the_unit_slot_in_typescript_and_javascript` |
| Map locations from the nearest `eudkit.toml` | PASS | PASS | PASS | `completes_map_locations_from_the_nearest_eudkit_toml`, `completes_map_locations_and_switches_in_typescript_and_javascript` |
| Named map switches plus `Switch 1`..`Switch 256` | PASS | PASS | PASS | `completes_map_switches_from_eudkit_toml` |
| Canonical fallback without a project | PASS | PASS | PASS | `falls_back_to_canonical_symbols_without_eudkit_toml` and language variant |
| Canonical fallback for invalid project/map | PASS | PASS | PASS | `falls_back_to_canonical_locations_for_invalid_projects_and_maps` |
| Nearest project wins; project/map changes reload | PASS | PASS | PASS | `nearest_project_wins_for_each_document`, `reloads_*` tests |
| Unknown unit/location/switch warning and clear-on-fix/close | PASS | PASS | PASS | `warns_about_unknown_map_symbols_*` |
| Custom map unit accepted by diagnostics | PASS | PASS | PASS | `accepts_known_custom_units_*` |
| Stock filters and numeric IDs accepted | PASS | PASS | PASS | `accepts_numeric_stock_filter_*` |
| EUD group-player warning above stock boundary | PASS | PASS | PASS | `warns_when_group_players_use_eud_offsets_*` |
| Concrete players, stock units, variables stay silent | PASS | PASS | PASS | `accepts_stock_and_ambiguous_arguments_*` |

### Hover and signature help (post-`v0.1.1`)

These rows were verified on the development tree after `v0.1.1`. The published
`v0.1.1` binaries do not advertise or implement these methods.

| Case | Python | TypeScript | JavaScript | Protocol tests |
| --- | --- | --- | --- | --- |
| Initialize advertises `hoverProvider` and `signatureHelpProvider` with `(` and `,` triggers | PASS | PASS | PASS | `initializes_with_hover_and_signature_help_capabilities` |
| Canonical unit hover shows canonical/map status and ID | PASS | PASS | PASS | `hovers_over_python_symbols_and_supported_calls`, `hovers_over_typescript_and_javascript_units_and_qualified_calls` |
| Stock Unit Filter hover shows the filter ID | PASS | n/a | n/a | `hovers_over_python_symbols_and_supported_calls` |
| Canonical location/switch hover | PASS | n/a | n/a | `hovers_over_python_symbols_and_supported_calls` |
| Map Context unit and location hover | PASS | n/a | n/a | `hovers_over_map_context_symbols_when_a_project_is_present` |
| Supported call hover shows signature, summary, return type, and symbol parameters | PASS | PASS | PASS | `hovers_over_python_symbols_and_supported_calls`, `hovers_over_typescript_and_javascript_units_and_qualified_calls` |
| Qualified `eudkit.KillUnit(...)` call hover | n/a | PASS | PASS | `hovers_over_typescript_and_javascript_units_and_qualified_calls` |
| Signature help inside a call and after a comma | PASS | PASS | PASS | `provides_python_signature_help_at_calls_and_keyword_arguments`, `provides_signature_help_for_qualified_typescript_and_javascript_calls` |
| Python keyword argument selects its parameter | PASS | n/a | n/a | `provides_python_signature_help_at_calls_and_keyword_arguments` |
| Unknown/non-call hover and unsupported call signature help return null, not an error | PASS | PASS | PASS | `returns_no_hover_or_signature_help_outside_supported_contexts` |

## VS Code reference matrix

This pass used a temporary `@vscode/test-electron` harness against the real
extension host, with the installed release VSIX bundle as the extension under
test. The workspace contained three folders:

- `plain/`: no `eudkit.toml`.
- `verify-project/`: `eudkit.toml` pointing at `test_maps/v2/verify_v2.scx`
  (one named location, no named switches).
- `basemap-project/`: `eudkit.toml` pointing at
  `tests/outputmap/basemap/사이비2.scx` (named locations and custom unit names).

Each language file was opened as a real VS Code document and completion was
requested through `vscode.executeCompletionItemProvider`. Diagnostics were read
through `vscode.languages.getDiagnostics`. Hover and signature help were added
after the `v0.1.1` VSIX under test was built, so they are covered by the shared
protocol matrix and not by this VS Code UI pass.

| Check | Python | TypeScript | JavaScript |
| --- | --- | --- | --- |
| Extension activates and server starts from the verified release download/cache | PASS | PASS | PASS |
| 228 canonical units, including Marine/Zergling/Zealot | PASS | PASS | PASS |
| No `Any unit`/`Men`/`Buildings`/`Factories` in unit completion | PASS | PASS | PASS |
| No custom `UNIx` names in unit completion | PASS | PASS | PASS |
| No-toml location completion is exactly `Anywhere` | PASS | PASS | PASS |
| No-toml switch completion is exactly 256 canonical switches | PASS | PASS | PASS |
| Named map location from `verify_v2.scx` appears | PASS | PASS | PASS |
| Map switch completion remains unique and canonical | PASS | PASS | PASS |
| `dvs1` named location from `사이비2.scx` appears | PASS | PASS | PASS |
| Custom map unit stays out of completion but is accepted by diagnostics | PASS | PASS | PASS |
| Unknown unit, location, and switch produce source `eudkit` warnings | PASS | PASS | PASS |
| Stock filters and numeric IDs produce no warnings | PASS | PASS | PASS |
| Group player + offset `300` produces the EUD warning | PASS | PASS | PASS |
| Concrete player + offset `300` stays silent | PASS | PASS | PASS |
| Warnings clear when the document is fixed | PASS | PASS | PASS |
| Hover and signature help UI (post-`v0.1.1`) | Not executed in this VSIX pass | Not executed in this VSIX pass | Not executed in this VSIX pass |
| No `eudkit.inputMap` setting; default `eudkit.lsp.path` is empty | PASS | PASS | PASS |

The hover/signature row is intentionally not a release-UI claim: the installed
VSIX in this pass was the published `v0.1.1` bundle, whose server does not
implement those methods. Their shared server behavior is covered by the
protocol matrix above.

Observed release-download record:

```text
Downloading required checksum https://github.com/Tb1281/eudplib/releases/download/v0.1.1/eudkit-lsp-linux-x86_64.sha256
Downloading https://github.com/Tb1281/eudplib/releases/download/v0.1.1/eudkit-lsp-linux-x86_64
Verified SHA-256 checksum for eudkit-lsp-linux-x86_64.
Installed eudkit-lsp 0.1.1 at .../globalStorage/eudkit.eudkit-vscode/eudkit-lsp/0.1.1/eudkit-lsp-linux-x86_64
Using eudkit-lsp 0.1.1 from GitHub Releases: ...
Language server started.
```

## Zed client matrix

| Check | Result | Evidence |
| --- | --- | --- |
| Dev extension manifest loads | PASS | `extensions updated. loading 2` (the profile also had an unrelated extension) |
| `eudkit-lsp` starts for `main.py` | PASS | `starting language server process ... eudkit-lsp` |
| `eudkit-lsp` starts for `main.ts` | PASS | `starting language server process ... eudkit-lsp` |
| `eudkit-lsp` starts for `main.js` | PASS | `starting language server process ... eudkit-lsp` |
| Language registration matches the shared server | PASS | `extension.toml`: Python, TypeScript, TSX, JavaScript; language IDs `python`, `typescript`, `typescriptreact`, `javascript` |
| Completion/diagnostics/hover/signature UI | **NOT EXECUTED** | Shared server behavior is covered by the protocol matrix; the extension contains no behavior or UI logic and the UI was not driven in this environment |

The Zed extension contains no UI code. Completion, hover, and signature results
are rendered by Zed's stock LSP client from the shared server responses; this
record verifies server startup and language registration, not Zed's rendering.

Zed version compatibility is structural rather than a runtime `--version`
probe: the extension pins the release URL and cache to `CARGO_PKG_VERSION`
(`0.1.1` → `v0.1.1`) and requires the published checksum. The `0.1.x`
compatibility rule for the VS Code client is enforced at runtime.

## Commands run

```sh
# Shared server: 45 lib + 4 main + 4 CLI + 34 protocol = 87 tests
(cd src/rust && cargo test -p eudkit-lsp)

# Clippy is clean for eudkit-lsp; eudkit-core has pre-existing warnings outside
# this change.
(cd src/rust && cargo clippy -p eudkit-lsp --all-targets)

# Real LSP smoke: version and 228 units for bare and --stdio invocations
python3 scripts/smoke_eudkit_lsp.py \
  src/rust/target/release/eudkit-lsp --expected-version 0.1.1

# VS Code extension unit, release, and type-fixture checks: 21 tests + tsc
(cd editors/vscode && npm test)

# Release VSIX install in a clean profile (no Rust)
sha256sum -c eudkit-vscode-0.1.1.vsix.sha256
code --user-data-dir <clean-profile> --extensions-dir <clean-extensions> \
  --install-extension eudkit-vscode-0.1.1.vsix
code --user-data-dir <clean-profile> --extensions-dir <clean-extensions> \
  --list-extensions --show-versions

# Zed extension tests: 17 tests
(cd editors/zed && cargo test)
```

The VS Code extension-host pass and the Zed dev-extension pass were run
manually for this record; their temporary profiles were outside the repository.

## Platform coverage

| Platform | Architecture | VSIX/editor | `eudkit-lsp` execution |
| --- | --- | --- | --- |
| Linux | x86_64 | PASS (clean VSIX install + extension host) | PASS (this record) |
| Windows | x86_64 | Built and published; not executed here | Built and checksum/format verified; not executed here |
| macOS | arm64 | Built and published; not executed here | Built and checksum/format verified; not executed here |
| macOS | x86_64 | Built and published; not executed here | Built and checksum/format verified; not executed here |

Supported combinations are Windows x64, Linux x64, macOS arm64, and macOS
x86_64. Other combinations require a local server override.

## Known verification gaps

- The VS Code behavior pass loaded the installed VSIX bundle through the
  extension-under-test path rather than activating it from the extension
  registry. The clean-profile `code --install-extension` and
  `--list-extensions` checks cover the install path; the behavior matrix covers
  the packaged extension and the release server download. The remaining
  clean-machine checklist is documented in
  [`editor-install-smoke-test.md`](editor-install-smoke-test.md).
- The Zed completion, hover, and signature UI was not driven programmatically
  because the Zed extension API has no test harness and the environment is
  Wayland-only. Zed client startup and language registration were verified, the
  extension contains no UI code, and the shared server behavior is covered by
  the protocol matrix.
- Windows and macOS binaries were not executed in this Linux environment.
- Pylance and tsserver were not installed in the temporary VS Code profile.
  Their coexistence is documented as a troubleshooting rule; the eudkit client
  was verified to publish only source `eudkit` warnings and to work without
  either language server present.

These gaps are explicit acceptance limits, not silent assumptions.
