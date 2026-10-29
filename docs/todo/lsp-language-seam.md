# TODO — Deepen the LSP language seam (deferred design)

Status: **design-only, deferred** (2026-10-29). This is a design record, not an
active spec. No code changes accompany it. It records candidate 7 of the
2026-10-29 architecture review and the conditions under which the seam should be
implemented. The current server is correct; the question is where its next
axis of growth should land.

Related records:

- [`docs/handoffs/2026-10-28-lsp-mvp.md`](../handoffs/2026-10-28-lsp-mvp.md) —
  #54 review dispositions and the LSP MVP state.
- [`GLOSSARY.md`](../../GLOSSARY.md) — Frontend, Map Context, Map Symbol,
  Canonical Symbol, Reactive Rule.
- Architecture review, candidate 7 — temporary report
  `/tmp/opencode/architecture-review-20261029-123254.html`, not part of the
  repository.

## Decision

Do **not** implement this seam yet. When it is revisited:

- Construct one `DocumentAnalysis` view from the LSP language id at the server
  edge. It exposes four operations: `completions`, `hover`, `signature`, and
  `diagnostics`.
- Keep two real adapters: the Python adapter over `ruff`, and the
  TypeScript/JavaScript adapter over `oxc`. The adapters own parser choice,
  token scanning, and per-language analysis.
- Keep `server.rs` responsible for LSP types, UTF-16 ranges, request and
  notification plumbing, and project lookup. It should not know which parser a
  Frontend uses.
- Keep `&str` at the edge. `Language::from_lsp(language_id: &str)` is the only
  place where an LSP language id becomes an internal language value.
- Keep `CallContext` and `CallSite` separate. They model different token
  sources and different questions; the seam does not merge them.
- Keep canonical resolution in `validation.rs`. Both adapters call it; it does
  not move into either adapter.
- Keep the small location/switch hover duplication. It is local to `hover.rs`
  and is not made worse by this seam.

Revisit only when **a third Frontend** or **a fifth LSP feature** lands. Until
then the four explicit dispatch sites are the cheaper, clearer shape.

## Why this is deferred

The current code has a real seam already: every per-language operation has a
Python function and a TypeScript function, and both return comparable data.
What it does not have is one place that owns the choice between them. That is
friction, but it is not yet expensive friction:

- There are only two adapters. A seam with two implementations is the minimum
  needed to prove the interface; extracting it earlier would produce a shallow
  module that mostly forwards calls.
- The four dispatch sites are short and colocated in `server.rs`. A reader can
  see every language branch in one screen.
- The #54 review explicitly accepted this shape: keep language selection as
  `&str`, keep `CallContext` and `CallSite` separate, keep the local hover
  duplication, and do not plan a further refactor. This proposal keeps all of
  those dispositions. It re-opens only "no further refactor planned", and only
  behind a future trigger.
- Nothing user-visible is blocked. Completion, diagnostics, hover, and
  signature help all work for every supported language id.

The seam becomes worth its own module when either axis grows:

- a third Frontend adds a third parser and a third set of scanners, so each
  feature would otherwise gain a third arm in four handlers;
- a fifth feature adds another `match language_id` door, so each Frontend
  would otherwise be wired in one more place.

At that point the multiplication is real: `features × Frontends` edits in
`server.rs`, plus parallel changes in `analysis/mod.rs`. The seam turns that
into `features + Frontends`.

## Current shape (before)

`server.rs` has four dispatch doors:

| Handler | Current branch | Python path | TypeScript path |
| --- | --- | --- | --- |
| completion | `completion_response` (`server.rs:154`) | `ParsedDocument::parse` → `completions_at_with_context` | `TsLanguage::from_language_id` → `ts_completions_at` |
| hover | `hover_response` (`server.rs:177`) | `python_hover` | `ts_hover` |
| signature help | `signature_help_response` (`server.rs:259`) | `python_call_context` | `ts_call_context` |
| diagnostics | `document_diagnostics` (`server.rs:373`) | `python_diagnostics` + `python_eud_player_diagnostics` | `ts_diagnostics` + `ts_eud_player_diagnostics` |

The current flow is:

```text
server.rs
  completion  ── match language_id ──> ParsedDocument  | TsLanguage
  hover       ── match language_id ──> python_hover    | ts_hover
  signature   ── match language_id ──> python_call_context | ts_call_context
  diagnostics ── match language_id ──> python_* + python_eud_* | ts_* + ts_eud_*

analysis/mod.rs re-exports every per-language entry point
```

Three details matter for the design:

1. **The return shapes are nearly uniform, but not quite.** Python symbol slots
   borrow the source through `ParsedDocument<'a>`; TypeScript symbol slots are
   owned `Copy` values. Both expose a content range, a literal range, and a
   `SymbolKind`. `CallContext` is already owned and shared. Diagnostics already
   share `SymbolDiagnostic` / `EudPlayerDiagnostic`.
2. **Map Context resolution is duplicated.** Completion resolves the project
   inline (`server.rs:150-153`), while hover uses
   `map_context_for_document` (`server.rs:301-309`). Signature help does not
   need the Map Context today. Diagnostics resolves it again
   (`server.rs:362-365`).
3. **Each request re-parses.** Completion, hover, signature help, and
   diagnostics each construct a parser or scan the source independently. The
   Python diagnostics path parses once for Map Symbols and scans again for EUD
   player warnings; the TypeScript path does the same.

None of this is a correctness bug. The friction is that `server.rs` is the
place where parser knowledge, feature behavior, and LSP conversion meet. A new
Frontend would be edited there four times; a new feature would add a fifth
match.

## Proposed shape (after)

One language value, one analysis view, two adapters:

```text
server.rs
  LSP types · UTF-16 ranges · project lookup · notification plumbing
        │
        │  DocumentAnalysis::new(language_id: &str, source, map_context)
        ▼
  Language::from_lsp(&str)          ← the only &str conversion
        │
        ├── Python adapter (ruff: ParsedDocument, tokens, CallSite)
        └── TypeScript adapter (oxc: TsLanguage, tokens, fallback scanner)
        │
        ▼
  completions · hover · signature · diagnostics
```

`server.rs` still converts the analysis results into `CompletionItem`, `Hover`,
`SignatureHelp`, and `Diagnostic`. `analysis` never imports `lsp_types`.

### Interface sketch

```rust
// analysis/language.rs
pub enum Language {
    Python,
    TypeScript(TsLanguage),
}

impl Language {
    /// The only place an LSP language id becomes an analysis language.
    pub fn from_lsp(language_id: &str) -> Option<Self> {
        match language_id {
            "python" => Some(Self::Python),
            other => TsLanguage::from_language_id(other).map(Self::TypeScript),
        }
    }
}

// analysis/document_analysis.rs
pub struct DocumentAnalysis<'a> {
    language: Language,
    source: &'a str,
    map_context: Option<&'a MapContext>,
}

impl<'a> DocumentAnalysis<'a> {
    pub fn new(
        language_id: &str,
        source: &'a str,
        map_context: Option<&'a MapContext>,
    ) -> Option<Self>;

    pub fn completions(&self, offset: usize) -> Option<SymbolCompletion>;
    pub fn hover(&self, offset: usize) -> Option<HoverInfo>;
    pub fn signature(&self, offset: usize) -> Option<CallContext>;
    pub fn diagnostics(&self) -> Vec<DiagnosticFinding>;
}

/// A symbol-typed string slot, in the shape the server needs for completion.
pub struct SymbolCompletion {
    pub replace: Range<usize>,
    pub kind: SymbolKind,
    pub names: Vec<String>,
}

pub struct HoverInfo {
    pub markdown: String,
    pub range: Range<usize>,
}

pub struct DiagnosticFinding {
    pub range: Range<usize>,
    pub message: String,
}
```

The interface is deliberately narrow and owned. The adapters may borrow
internally (Python's `ParsedDocument<'a>`, oxc's allocator), but the server sees
no parser lifetimes and no parser types. `SymbolCompletion` replaces the current
`(source, content_range, kind)` tuple that `completion.rs` turns into LSP items.

### Why an enum, not a trait object

The Frontend set is closed at compile time: Python, TypeScript, TSX,
JavaScript, and JSX, with the last four sharing one adapter. An enum gives
exhaustive wiring: adding a Frontend fails to compile until every operation has
an arm. A trait object would make the set open, but it would also make
"forgot to wire the new Frontend into diagnostics" a runtime omission unless a
registry test catches it.

The seam is the constructor and the four operations, not a type hierarchy. If a
third Frontend later arrives as a plugin with an open set, the same
`DocumentAnalysis` interface can be implemented over a trait object without
changing `server.rs`. That is a later decision; it should not be paid for now.

### Where each concern lives

| Concern | After the seam |
| --- | --- |
| LSP request parsing, ranges, capabilities | `server.rs` (unchanged) |
| Project discovery and Map Context cache | `project.rs` (unchanged) |
| Language id → adapter | `Language::from_lsp` |
| Python tokenization and symbol slots | Python adapter (`ParsedDocument`, `symbol_slot`) |
| TypeScript/JavaScript tokenization and fallback | TypeScript adapter (`analysis/ts.rs`) |
| Call context for signature help | Adapter returns `CallContext`; `CallSite` stays internal to Python call scanning |
| Canonical and Map Symbol resolution | `analysis/validation.rs` (unchanged) |
| Function documentation tables and language rendering | `analysis/function_docs.rs` (unchanged) |
| Completion item construction | `completion.rs`, fed by `SymbolCompletion` |
| Diagnostic severity, source, sorting | `server.rs` (sorting may stay in the analysis view) |

This placement preserves the #54 dispositions:

- `CallContext` and `CallSite` remain separate types. `DocumentAnalysis` exposes
  `CallContext` because that is what signature help consumes; `CallSite` stays a
  Python-internal parsing detail.
- `&str` remains at the edge. The server still stores
  `Document.language_id: String` for function documentation, and
  `Language::from_lsp` is the only conversion. No LSP type changes.
- Canonical resolution remains in `validation.rs`, imported by both adapters.
- Location/switch hover duplication remains in `hover.rs`, called by both
  adapters. The seam does not force a shared hover helper.
- The symbol and documentation tables remain separate, with the existing
  table-sync tests. The seam does not merge them.

### Map Context

The server resolves the Map Context once per request through
`map_context_for_document` and passes it into `DocumentAnalysis`. This removes
the inline resolution in `completion_response` and the second resolution in
`document_diagnostics`. The analysis view receives a borrowed `MapContext`; it
does not own or reload it. Project caching and file-change detection remain in
`project.rs`.

The first implementation does not add cross-request parse caching. The
interface permits it later: a future `Document` could hold a parsed analysis
keyed by document version, while the Map Context is still passed per request.
That is an implementation change behind the interface, not a new seam.

## Behavior contract

These properties must hold before and after the seam. They are the parity
oracle for the migration.

| Property | Required behavior |
| --- | --- |
| Supported language ids | `python`; `typescript`/`ts`; `typescriptreact`/`tsx`; `javascript`/`js`; `javascriptreact`/`jsx` |
| Unknown language id | Completion, hover, and signature help return no result; diagnostics publish an empty list; no panic |
| Completion | Symbol slot only; Map Symbols when a Map Context exists, Canonical Symbols otherwise |
| Hover order | Symbol hover first, supported-call hover second; literal range for symbols, identifier range for calls |
| Signature help | `CallContext` lookup first; `function_doc` and language-specific parameter names after; keyword arguments keep priority over positional index |
| Diagnostics | Map Symbol warnings plus EUD player warnings; sorted by start offset; severity `WARNING`; source `eudkit`; cleared on fix and on close |
| Ranges | Analysis stays in byte offsets; `server.rs` converts to UTF-16 LSP positions exactly as today |
| Protocol | No capability, trigger-character, or notification changes |

## Migration plan

Each step is independently shippable and reversible. None changes the public
CLI, the editor packages, or the wire protocol.

1. **Freeze behavior.** Keep the 35 protocol tests as the oracle. Add a
   language-id matrix test for `Language::from_lsp`, including every alias and
   an unknown id. No production behavior change.
2. **Introduce `Language`.** Add `analysis/language.rs`; re-export it from
   `analysis/mod.rs`. Replace `TsLanguage::from_language_id` at the four server
   doors with `Language::from_lsp`, still branching as today. This proves the
   conversion without moving analysis.
3. **Extract the Python adapter.** Move `python_hover`, the Python completion
   path, `python_call_context`, and the Python diagnostics pair behind uniform
   owned outputs. Keep the old free functions as thin wrappers until the server
   switches. Run the protocol tests unchanged.
4. **Extract the TypeScript adapter.** Do the same for `ts_hover`,
   `ts_completions_at`, `ts_call_context`, and the TypeScript diagnostics pair.
   Keep the oxc fatal-error fallback and the unterminated-string scan intact.
5. **Introduce `DocumentAnalysis`.** Switch one handler at a time, in this
   order: signature help (already `CallContext`-shaped), hover, completion,
   diagnostics. After each switch, run the protocol matrix and compare exact
   LSP responses for a representative Python and TypeScript document.
6. **Remove the old wrappers and per-language re-exports.** Leave only the
   `DocumentAnalysis` interface and the adapter modules. Update the handoff and
   this record's status.

The ordering keeps the blast radius small: steps 1–2 are mechanical, steps 3–4
move code without changing callers, and step 5 changes one door at a time.

## Tests

- **Language matrix.** Every supported alias maps to the expected `Language`;
  unknown ids return `None`; `tsx`/`jsx` map to the TypeScript adapter with the
  right `TsLanguage`.
- **Handler parity.** For each feature, build the same document through the old
  path and the new path and compare the resulting LSP value, not just "some
  result". Cover at least: canonical unit completion, Map Context location
  completion, switch completion, symbol hover, qualified call hover, Python
  keyword-argument signature help, TypeScript comma-advanced signature help,
  unknown-symbol diagnostics, and EUD-player diagnostics.
- **Cross-language matrix.** For each feature, run the supported language ids
  that the feature covers. This is the test that fails when a future Frontend is
  half-wired.
- **Regression.** The existing suite remains: 46 lib + 4 main + 4 CLI + 35
  protocol tests at the #54 tree. No protocol test should need editing for the
  seam; if one does, that is a behavior change and needs its own justification.
- **Static checks.** `cargo fmt -p eudkit-lsp -- --check`,
  `cargo clippy -p eudkit-lsp --all-targets`, and the release smoke script
  (`scripts/smoke_eudkit_lsp.py`) stay green.

## Risks and mitigations

| Risk | Mitigation |
| --- | --- |
| Borrowed Python slots vs owned TypeScript slots force a lowest-common-denominator type | The interface returns owned ranges and `SymbolKind`; parser lifetimes stay inside adapters |
| Behavior drift in the TypeScript fatal-error fallback | Keep `fallback_ts_call_context` and the unterminated-string slot scan as adapter internals; add parity cases for both |
| Python unterminated strings and partial literals behave differently | Keep `ParsedDocument` and the existing token logic; completion/hover parity tests cover the partial cases |
| Map Context reload timing changes | Resolve once per request with the existing `ProjectCache`; pass a borrowed context; do not cache it in the analysis view |
| Unknown-language handling becomes inconsistent | One `Language::from_lsp`; the server maps `None` to the current per-feature no-result behavior |
| Function documentation starts depending on the adapter | Keep `function_docs.rs` outside the seam; the server renders with `Document.language_id` as today |
| The abstraction becomes shallow forwarding | Only introduce it when the third Frontend or fifth feature makes the dispatch multiplication real; keep the interface at four operations |
| Protocol tests become duplicated per language | Parameterize the cross-language matrix; keep the existing 35 tests as the behavioral oracle |

## Revisit criteria

Implement this seam when one of these is true:

1. A **third Frontend** lands — a new language id with its own parser or token
   model, beyond the TypeScript/TSX/JavaScript/JSX family.
2. A **fifth LSP feature** lands — document symbols, go-to-definition,
   references, code actions, formatting, or any new request/notification that
   needs per-language analysis.
3. A second consumer (the CLI, an editor package, or a test harness) needs the
   same per-language analysis outside the server.
4. The four dispatch doors grow beyond four, or a handler needs a third
   language-specific branch for a reason other than a new Frontend.

Do not implement it for a bug fix, a documentation change, or a new supported
call in `function_docs.rs`. Those are local edits and do not change the
multiplication.

## References

- `src/rust/eudkit-lsp/src/server.rs` — the four dispatch doors:
  completion `142-165`, hover `167-229`, signature `247-299`, diagnostics
  `353-418`.
- `src/rust/eudkit-lsp/src/completion.rs` — completion item construction and
  the Python/TypeScript entry points.
- `src/rust/eudkit-lsp/src/analysis/mod.rs` — per-language re-exports.
- `src/rust/eudkit-lsp/src/analysis/document.rs` — Python `ParsedDocument`.
- `src/rust/eudkit-lsp/src/analysis/ts.rs` — TypeScript/JavaScript adapter and
  `TsLanguage`.
- `src/rust/eudkit-lsp/src/analysis/signature.rs` — `CallContext` and the two
  call scans.
- `src/rust/eudkit-lsp/src/analysis/call_site.rs` — Python `CallSite`.
- `src/rust/eudkit-lsp/src/analysis/validation.rs` — shared canonical and Map
  Symbol resolution.
- `src/rust/eudkit-lsp/src/analysis/function_docs.rs` — function documentation
  tables and language-specific rendering.
- `src/rust/eudkit-lsp/tests/protocol.rs` — 35 protocol tests; the behavioral
  oracle for the migration.
- `docs/handoffs/2026-10-28-lsp-mvp.md` — #54 dispositions and verification
  counts.
