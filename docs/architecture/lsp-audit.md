# eudkit-lsp Scope Audit and Domain-Feature Plan

**Status:** Proposed
**Baseline:** `feat/lsp-mvp` at `27fac82c`
**Decision record:** [ADR-0006](../adr/0006-frozen-hir-plans.md) (on `eudkit`
until the branches merge)
**Companion plan:** `docs/architecture/cdk-synthesis-tooling-plan.md` (on
`eudkit`)

## 1. Executive summary

`eudkit-lsp` should be a **domain language server**, not a second general
language server. It already has the right core:

- Map Context completion for canonical units, locations, and switches.
- Symbol hover that resolves a name to an ID and says whether it is canonical
  or map-defined.
- Unknown Map Symbol diagnostics.
- EUD group-player diagnostics.
- Project discovery and Map Context loading from `eudkit.toml`.
- Python, TypeScript, TSX, JavaScript, and JSX analysis.

It also has a layer that duplicates what Pyright/basedpyright and
tsserver/vtsls already do:

- Call hover for 16 frontend functions.
- Signature help for those calls, including keyword and qualified-call
  handling.
- `function_docs.rs` and `signature.rs` to support both.

**Recommendation:**

| Action | Feature |
| --- | --- |
| Keep | Completion, symbol hover, Map Symbol diagnostics, EUD-player diagnostics, project discovery, TS/JS parsing |
| Remove | Call hover, signature help, `function_docs.rs`, `signature.rs` |
| Keep capability | `hoverProvider` (symbol hover remains; offset hover is added) |
| Remove capability | `signatureHelpProvider` |
| Add | Map Context inspection request, unit/weapon/constant IDs, EUD offset annotations, plan-based trigger/VProc diagnostics |

This is a deliberate breaking change to the LSP surface. Symbol hover stays
because no host language server can resolve a map-defined unit name to a
StarCraft ID. Call hover and signature help go because the SDK's `.pyi` and
`.d.ts` files are the intended source for that information in the host
language server.

### Branch reality check

`feat/lsp-mvp` (`27fac82c`) contains #54, which introduced hover and signature
help. `eudkit` (`9986c871`) contains #55 and the frozen-plan work but branched
before #54, so its LSP tree is older, not a pruned version of this one. Do not
use the `eudkit` LSP directory as evidence that these modules were already
removed. Merge the plan branch first, then apply this audit to the merged tree.

## 2. Current capability inventory

| Capability | State on `feat/lsp-mvp` | Source | Decision |
| --- | --- | --- | --- |
| Canonical unit completion | Present | `completion.rs`, `symbol_slot.rs`, `map_context.rs` | Keep |
| Map location/switch completion | Present | `completion.rs`, `project.rs` | Keep |
| Symbol hover (unit/location/switch) | Present | `analysis/hover.rs` | Keep |
| Call hover | Present | `analysis/hover.rs`, `analysis/function_docs.rs` | Remove |
| Signature help | Present | `analysis/signature.rs`, `server.rs` | Remove |
| Unknown Map Symbol diagnostics | Present | `analysis/validation.rs` | Keep |
| EUD group-player diagnostics | Present | `analysis/eud_player.rs` | Keep |
| Project discovery / Map Context cache | Present | `project.rs` | Keep, extend |
| Python parsing | Present | `analysis/document.rs` | Keep |
| TypeScript/JS/TSX/JSX parsing | Present | `analysis/ts.rs` | Keep |
| Map Context inspection | Absent (CLI only) | `eudkit inspect` | Add |
| Unit/weapon/constant IDs | Partial (unit strings only) | `hover.rs`, `map_context.rs` | Add |
| EUD memory offset annotations | Absent | `hir.rs`, `types.py` | Add |
| Trigger-limit diagnostics | Absent | `lir/trigger_emitter.rs` | Add |
| VProc diagnostics | Absent | `lir/storage_selection.rs` | Add |
| Generic type checking | Absent by design | Host language server | Never add |

## 3. File-by-file audit

Line numbers refer to `27fac82c` and will shift after the merge. Treat them as
anchors, not stable identifiers.

### 3.1 `src/rust/eudkit-lsp/src/server.rs` (483 lines)

| Lines | Content | Decision |
| --- | --- | --- |
| 49-55 | `text_document_sync`, `completion_provider`, `hover_provider` | Keep |
| 56-60 | `signature_help_provider` and its trigger characters | Remove |
| 99-121 | Completion and hover request dispatch | Keep |
| 122-133 | `textDocument/signatureHelp` dispatch | Remove |
| 142-165 | `completion_response` | Keep |
| 167-196 | `hover_response` | Keep |
| 198-214 | `python_hover`; symbol branch | Keep symbol branch, remove `function_hover_at` fallback at 213 |
| 216-229 | `ts_hover`; symbol branch | Keep symbol branch, remove `function_hover_at` fallback at 228 |
| 231-245 | `function_hover_at` | Remove |
| 247-299 | `signature_help_response` | Remove |
| 301-309 | `map_context_for_document` | Keep |
| 311-351 | Document open/change/close notifications | Keep |
| 353-418 | `publish_diagnostics` and `document_diagnostics` | Keep |
| 420-435 | `send_diagnostics` | Keep |
| 437-483 | UTF-16 position conversion and test | Keep |

Import cleanup:

- Remove `function_doc`, `function_hover`, `identifier_at`,
  `python_call_context`, and `ts_call_context` from the `analysis` import at
  lines 8-10.
- Remove `Documentation`, `ParameterInformation`, `ParameterLabel`,
  `SignatureHelp`, `SignatureHelpOptions`, `SignatureHelpParams`, and
  `SignatureInformation` from the `lsp_types` import.
- Keep `Hover`, `HoverContents`, `HoverParams`, `HoverProviderCapability`,
  `MarkupContent`, and `MarkupKind`.

The resulting server advertises:

```json
{
  "textDocumentSync": 1,
  "completionProvider": {
    "triggerCharacters": ["\"", "'", "("]
  },
  "hoverProvider": true
}
```

No `signatureHelpProvider`.

### 3.2 `src/rust/eudkit-lsp/src/analysis/hover.rs` (170 lines)

| Lines | Content | Decision |
| --- | --- | --- |
| 1-7 | Module docs; mentions "and calls" | Keep, reword to symbol-only |
| 8-12 | Imports | Keep `map_context`; remove `function_docs` import at 10 |
| 14-17 | Stock Unit Filter range | Keep |
| 19-113 | `symbol_hover`, `unit_hover`, `location_hover`, `switch_hover` | Keep |
| 115-118 | `function_hover` | Remove |
| 120-158 | Symbol hover tests | Keep |
| 159-169 | Call hover test | Remove |

This file remains the domain-hover implementation. Rename it to
`symbol_hover.rs` if the module list becomes confusing; not required.

### 3.3 `src/rust/eudkit-lsp/src/analysis/function_docs.rs` (351 lines)

Delete the file. It exists only for call hover and signature help. Its tests
also enforce a coupling between `PYTHON_SYMBOLS` and the documentation table;
that coupling disappears with the feature.

### 3.4 `src/rust/eudkit-lsp/src/analysis/signature.rs` (346 lines)

Delete the file. It locates the call under the cursor, tracks commas and
keyword arguments, and builds `CallContext` for signature help. No remaining
feature needs it.

**Dependency correction:** `symbol_slot.rs` does not import `signature.rs`.
It imports `call_site.rs` directly at line 7. `call_site.rs` is the shared
call/argument locator used by symbol completion and validation. Therefore:

- Delete `signature.rs`.
- Keep `call_site.rs`.
- Do not remove `call_site.rs` when removing signature help.

### 3.5 `src/rust/eudkit-lsp/src/analysis/call_site.rs` (120 lines)

Keep. It is the token-level call and argument locator used by `symbol_slot.rs`
and, transitively, by completion and validation.

### 3.6 `src/rust/eudkit-lsp/src/analysis/symbol_slot.rs` (314 lines)

Keep. This is the shared table of functions and arguments that accept unit,
location, or switch symbols. It is the source of truth for completion and
unknown-symbol validation.

### 3.7 `src/rust/eudkit-lsp/src/analysis/document.rs` (117 lines)

Keep. Python tokenization and string-literal extraction.

### 3.8 `src/rust/eudkit-lsp/src/analysis/ts.rs` (805 lines)

Keep. TypeScript/JavaScript/TSX/JSX analysis. Update the comment at line 234
that refers to "symbol and signature analysis" so it no longer mentions a
removed feature.

### 3.9 `src/rust/eudkit-lsp/src/analysis/validation.rs` (257 lines)

Keep. Unknown Map Symbol validation for Python and TypeScript/JavaScript.

### 3.10 `src/rust/eudkit-lsp/src/analysis/eud_player.rs` (705 lines)

Keep. EUD group-player warnings are domain diagnostics that no host language
server can produce.

### 3.11 `src/rust/eudkit-lsp/src/completion.rs` (152 lines)

Keep. Completion items and UTF-16 position conversion.

### 3.12 `src/rust/eudkit-lsp/src/project.rs` (210 lines)

Keep. Extend it with a cached plan lookup for the diagnostics described in
section 5.4. The existing cache already invalidates on `eudkit.toml` or input
map mtime; add plan mtime to the same key.

### 3.13 `src/rust/eudkit-lsp/src/analysis/mod.rs` (27 lines)

| Lines | Content | Decision |
| --- | --- | --- |
| 3-5 | `call_site`, `document`, `eud_player` | Keep |
| 6 | `function_docs` | Remove |
| 7 | `hover` | Keep |
| 8 | `signature` | Remove |
| 9-11 | `symbol_slot`, `ts`, `validation` | Keep |
| 13-18 | Re-exports for retained modules | Keep |
| 19 | `function_docs` re-exports | Remove |
| 20 | `hover::{function_hover, symbol_hover}` | Keep only `symbol_hover` |
| 21 | `signature` re-exports | Remove |

### 3.14 `src/rust/eudkit-lsp/tests/protocol.rs` (1870 lines)

| Lines | Content | Decision |
| --- | --- | --- |
| 196-242 | `hover`, `hover_for`, `signature_help`, `signature_help_for` helpers | Remove |
| 331-335 | `hover_markdown` helper | Remove if no hover assertions remain; symbol hover tests can assert the string directly |
| 1143-1160 | `initializes_with_hover_and_signature_help_capabilities` | Rewrite: assert `hoverProvider` true and `signatureHelpProvider` absent |
| 1162-1214 | Python symbol and call hover | Keep symbol assertions; remove call hover assertion |
| 1216-1245 | Map Context symbol hover | Keep |
| 1247-1280 | TS/JS unit and call hover | Keep unit hover; remove call hover |
| 1282-1309 | Legacy qualified completion and hover | Keep completion; remove hover |
| 1311-1390 | Python signature help | Remove |
| 1392-1465 | Qualified TS/JS signature help | Remove |
| 1467-1526 | No hover/signature outside supported contexts | Replace with a domain-boundary test: call hover is null, signature help is unsupported |
| 1528-1869 | Unknown-symbol and EUD-player diagnostics | Keep |

Add tests:

- `initialize` does not advertise `signatureHelpProvider`.
- Hovering a supported call name (`KillUnit`) returns null, proving call hover
  is gone.
- Hovering a canonical unit still returns its ID.
- `textDocument/signatureHelp` returns `MethodNotFound` or null, depending on
  the chosen compatibility policy. Prefer not advertising the capability and
  returning `MethodNotFound` so clients do not call it.

### 3.15 Editor packages

No code changes are required. VS Code and Zed start the same `eudkit-lsp` and
render whatever capabilities it advertises. Verify both extensions still start
the server after the capability change; add a release note that signature help
is gone and host language server signatures remain.

### 3.16 Documentation

Update:

- `docs/eudkit-lsp-user-guide.md`: remove call hover and signature help
  throughout, especially lines 6-11, 28-36, 46-51, 256-326, 472, 483-495, and
  544-552; keep symbol hover and explain the domain boundary; update install
  guidance from `eudplib` to the standalone `eudkit` Python SDK.
- `docs/eudkit-lsp-acceptance-matrix.md`: remove or rewrite every
  signature-help row and the `initialize` capability assertion (lines 5, 10,
  14, 45-46, 50-52, 57, 66, 71, 89-99, 107, and 129 onward); add symbol-hover,
  offset, and trigger-diagnostic rows.
- `docs/handoffs/2026-10-28-lsp-mvp.md`: leave as historical record; add a new
  handoff for the scope change rather than rewriting history.

## 4. Why these removals are safe

- `function_docs.rs` is imported only by `hover.rs` and `server.rs`. Grep
  confirms no other user.
- `signature.rs` is imported only by `server.rs` and its own tests.
- `call_site.rs` is independent; it remains for symbol slots.
- The editor extensions contain no hover/signature logic. They only download
  and start `eudkit-lsp`.
- The SDK `.pyi`/`.d.ts` surfaces (shipped by the companion plan) are the
  better source for call signatures and parameter names. Duplicating them in
  the LSP creates a second table that drifts from the SDK.
- Removing `signatureHelpProvider` is a capability change, not a protocol
  break: clients that do not see the capability do not call the method.

The one real cost is user-visible: call hover was shipped in `v0.1.2`. Treat
the removal as a minor release with explicit release notes, and keep symbol
hover so the domain value remains.

## 5. Domain features to add

### 5.1 Map Context inspection

The CLI already exposes this data:

```bash
eudkit inspect basemap.scx --dts map.d.ts
```

`src/rust/eudkit-cli/src/main.rs:66-147` prints strings, locations, placed
units, CUWP slots, unit settings, switches, players, and TRIG triggers.

Add an LSP request:

```text
eudkit/mapContext
params: { textDocument: { uri } }
result: {
  map: string,
  fingerprint: string,
  strings: number,
  locations: { total: number, named: number, anywhere: 64 },
  placedUnits: number,
  cuwpSlots: { total: number, inUse: number },
  unitSettings: { total: number, customized: number },
  switches: { total: number, named: number },
  players: { slots: number, active: number },
  triggers: number,
  named: {
    locations: string[],
    switches: string[],
    units: string[]
  }
}
```

Implementation:

- Reuse `ProjectCache::map_context_for_document`; do not spawn the CLI.
- Add `MapContext::summary()` in `eudkit-core` so CLI and LSP share the counts.
- Cache the summary with the Map Context; invalidate on `eudkit.toml` or map
  mtime.
- VS Code: add a command `eudkit.inspectMapContext` that sends the request and
  shows the JSON in an output channel or information message.
- Zed: no custom command is required for the first version; the request is
  still available to any client.

Tests: protocol request with a fixture map, no project, unreadable map, and
map replacement.

### 5.2 Unit, weapon, and constant IDs

Current symbol hover resolves string names. Extend it to numeric and
identifier forms:

| Source form | Hover |
| --- | --- |
| `KillUnit("Terran Marine", P1)` | `Canonical unit · ID 0` (existing) |
| `KillUnit(0, P1)` | `Terran Marine · unit ID 0` |
| `Deaths(P1, AtLeast, 1, 300)` | `unit field 300 · EUD offset` when the player is concrete |
| `SetTo`, `Add`, `Subtract` | `Modifier · 7/8/9` |
| `AtLeast`, `AtMost`, `Exactly` | `Comparison · 0/1/10` |
| `P1`..`P12`, `Foes`, `AllPlayers` | `Player constant · value` |

Weapon IDs need a table. `eudkit-core` has canonical units and constants but
no weapon-name table. The legacy data exists in
`src/eudplib/scdata/weapon.py`; do not import Python from the LSP. Generate a
small Rust table in `eudkit-core` (or a dedicated `scdata` module) from that
source, with a test that the lengths and known IDs match. Weapon hover then
works in weapon-typed arguments and on `Weapon` constants.

Rules:

- Numeric hover only fires when the argument position is known to accept a
  unit, weapon, comparison, modifier, or player. Do not annotate arbitrary
  integers.
- In a `Deaths`/`SetDeaths` unit field, a value above the stock unit range is
  an EUD offset, not a unit ID. Reuse the logic in `eud_player.rs` and
  `map_context.rs`.
- Keep the existing "numeric values are valid but unresolved" behavior for
  unknown contexts; do not turn every number into a hover.

### 5.3 EUD memory offset annotations

Add hover for known virtual addresses and offset arithmetic. The table belongs
in `eudkit-core`, not in the LSP:

| Address | Meaning | Notes |
| --- | --- | --- |
| `0x0058A364` | Deaths table base | EPD 0; `EPD(x) = (x - 0x58A364) / 4` |
| `0x006509B0` | CurrentPlayer cursor (`CP`) | Player 13 base |
| `0x0057F23C` | Game tick | |
| `0x0051A280` | PTS base | Player trigger structure |
| `0x0058DC60` | MRGN runtime base | |
| `0x191943C8` | STRx base | Relocated strings |
| `0x0059CCA8` | CUnit table base | 336-byte stride |

CUnit field examples from `src/eudkit/types.py:1965-2110`:

| Field | Byte offset | CUnit-relative EPD offset |
| --- | --- | --- |
| `hp` | `+0x008` | `+2` |
| `sprite` | `+0x00C` | `+3` |
| `move_target` | `+0x014` | `+5` |
| `pos` | `+0x028` | `+10` |

Annotation format:

```text
0x0059CCA8 + 0x008
→ CUnit.hp
  byte offset +0x008, CUnit-relative EPD offset +2
```

```text
0x0058A364
→ deaths table base (EPD 0)
  This is not the CUnit table. CUnit starts at 0x0059CCA8.
```

Do not repeat the incorrect example that maps `0x59CCA8` to the deaths table.
`0x59CCA8` is the CUnit table base; `0x0058A364` is the deaths table base.
Validate every address in the table against `eudkit-core/src/hir.rs` and
`src/eudkit/types.py` before shipping.

Implementation:

- Hover on hex integer literals and on known constant identifiers.
- Recognize `base + offset` and `base - offset` only when `base` is a known
  table and `offset` is a small, known field offset.
- Show the byte offset and EPD offset when both are meaningful.
- Do not annotate a bare address with no known table; return null.
- Add a `MemoryOffset` table with `address`, `name`, `kind`, `byte_offset`,
  `epd_offset`, and `note`.

### 5.4 Pre-compile trigger-limit and VProc diagnostics

Current behavior:

- `src/rust/eudkit-core/src/lir/trigger_emitter.rs:1324` rejects more than 16
  conditions.
- `:1329` rejects more than 64 lowered actions.
- `src/rust/eudkit-core/src/allocator.rs:326` assumes at most 64 actions.
- `src/rust/eudkit-core/src/lir/storage_selection.rs:52` identifies variables
  that require 72-byte VProc slots.

The LSP should surface these before a compile, from the plan:

1. Add `eudkit-core::diagnostics`:

   ```rust
   pub enum Severity { Error, Warning, Information }

   pub struct Diagnostic {
       pub code: &'static str,
       pub severity: Severity,
       pub message: String,
       pub hir: Option<HirRef>,
       pub source: Option<SourceSpan>,
   }

   pub fn diagnose_plan(plan: &HirPlan, map: &MapContext) -> Vec<Diagnostic>;
   ```

2. Run the same HIR lowering that `compile` uses, but stop before allocation
   and map writing. Count per-trigger conditions and lowered actions; collect
   VProc promotions. Do not duplicate the thresholds in the LSP.

3. Diagnostic codes:

   | Code | Severity | Meaning |
   | --- | --- | --- |
   | `EUD-TRIG-001` | Error | Trigger has more than 64 lowered actions |
   | `EUD-TRIG-002` | Error | Trigger has more than 16 conditions |
   | `EUD-VPROC-001` | Warning | Variable is promoted to a 72-byte VProc slot |

4. Load the plan from the project:
   - Add `build.plan` to `eudkit.toml`, or discover
     `output_map.with_extension("eudhir")`.
   - If no plan exists, do not guess; run source-only diagnostics as today.
   - Cache by plan path, mtime, and fingerprint. Re-run on `didOpen` and
     `didSave`, not on every keystroke.

5. Source anchoring:
   - The current plan has no per-statement source spans, so anchor diagnostics
     at the plan file or at a trigger label.
   - Add optional `source` metadata (ADR-0006) and then per-statement spans.
     Only after that can diagnostics underline the source line that created
     the trigger.
   - Until then, publish a single plan-level diagnostic with the trigger label
     and action count; do not pretend to know a source line.

6. CLI parity:
   - `eudkit compile --plan` remains the authoritative check.
   - Add `eudkit check --plan <path>` (or `--check` on compile) that prints the
     same diagnostics without writing a map. The LSP and CLI must not drift.

Tests:

- A plan with a trigger that lowers to 65 actions reports `EUD-TRIG-001`.
- A plan with 17 conditions reports `EUD-TRIG-002`.
- A variable requiring VProc reports `EUD-VPROC-001`.
- A valid plan reports nothing.
- A plan with no fingerprint still diagnoses, with a warning.
- A changed plan invalidates the cache.

## 6. Role separation

| Question | Answer |
| --- | --- |
| Where do I see a function signature? | Host language server tooltip from `.pyi`/`.d.ts` |
| Where do I see a type error? | Pyright/basedpyright or tsserver/vtsls |
| Where do I see a unit's ID? | `eudkit-lsp` symbol hover |
| Where do I see a map location's bounds? | `eudkit-lsp` symbol hover |
| Where do I see a memory address's meaning? | `eudkit-lsp` offset hover |
| Where do I see "this trigger has 65 actions"? | `eudkit-lsp`, backed by core diagnostics |
| Where do I see the authoritative compile error? | `eudkit compile --plan` |
| Who owns Map Context discovery? | `eudkit-lsp` via `eudkit.toml` |

The LSP should never infer general types, resolve arbitrary imports, or
reimplement the SDK's documentation tables.

## 7. Rollout

1. Merge `eudkit` into `feat/lsp-mvp` (or rebase the LSP work onto `eudkit`).
2. Before removing call hover and signature help, land the Python `.pyi` and
   TypeScript `.d.ts` surfaces from the companion plan so host language
   servers can replace them. Then apply the prune patch:
   - Delete `function_docs.rs` and `signature.rs`.
   - Trim `hover.rs`, `server.rs`, `analysis/mod.rs`, and `tests/protocol.rs`.
   - Update the user guide and acceptance matrix.
3. Run `cargo test -p eudkit-lsp` and the protocol tests.
4. Add the domain features in this order:
   - Map Context inspection (lowest risk, reuses existing cache).
   - Unit/weapon/constant hover (tables in core).
   - EUD offset hover (table in core).
   - Plan diagnostics (needs core diagnostics and plan cache).
5. Release as a minor version, with release notes that call out the removed
   call hover and signature help.

Suggested version: `0.2.0`, because a shipped capability is removed. The editor
extensions do not need a version bump for protocol reasons, but should be
released together so the documentation and binaries match.

## 8. Acceptance criteria

- [ ] `initialize` advertises `completionProvider` and `hoverProvider`; it does
      not advertise `signatureHelpProvider`.
- [ ] Canonical and Map Context symbol hover still returns IDs and bounds.
- [ ] Hovering a supported call name returns null.
- [ ] `textDocument/signatureHelp` is not advertised and returns
      `MethodNotFound` if called.
- [ ] All existing completion and unknown-symbol/EUD-player diagnostics tests
      pass.
- [ ] `eudkit/mapContext` returns counts and named symbols for a fixture map.
- [ ] Numeric unit hover and constant hover resolve known IDs.
- [ ] EUD offset hover annotates `0x0058A364`, `0x0059CCA8`, and CUnit fields
      correctly.
- [ ] A 65-action plan reports `EUD-TRIG-001`; a 17-condition plan reports
      `EUD-TRIG-002`; a VProc variable reports `EUD-VPROC-001`.
- [ ] No Python or Node.js runtime is required by the LSP.
- [ ] The user guide and acceptance matrix describe the domain-only surface.
- [ ] Release notes explain the removal and point users to host language
      server signatures.

## 9. Risks

| Risk | Impact | Mitigation |
| --- | --- | --- |
| Removing shipped call hover/signature help | User surprise | Minor version, release notes, keep symbol hover |
| `eudkit` branch merge conflicts with #54 | Lost work or stale prune | Merge plan branch first; rerun audit on the merged tree |
| Plan has no source spans | Diagnostics cannot underline source | Phase diagnostics behind optional source metadata |
| Lowering in the LSP is expensive | Editor lag | Cache by plan mtime/fingerprint; run on open/save |
| Duplicate unit/weapon tables | Drift from compiler | Single table in `eudkit-core`, generated and tested |
| Wrong offset examples | Misleading documentation | Validate every address against `hir.rs` and `types.py` |
| `signatureHelpProvider` still advertised by an old client cache | Confusing UI | Capability is server-driven; restart the server after upgrade |

## 10. References

- ADR-0006: Host-Executed Frontends with Frozen HIR Plans.
- `docs/architecture/cdk-synthesis-tooling-plan.md`.
- `src/rust/eudkit-lsp/src/server.rs`.
- `src/rust/eudkit-lsp/src/analysis/`.
- `src/rust/eudkit-core/src/hir.rs`.
- `src/rust/eudkit-core/src/lir/trigger_emitter.rs`.
- `src/rust/eudkit-core/src/lir/storage_selection.rs`.
- `src/eudkit/types.py`.
- `src/eudplib/scdata/weapon.py`.
