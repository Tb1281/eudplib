# Host-Executed Frontends with Frozen HIR Plans

**Status:** Accepted for implementation; validation is tracked in #56.
**Addendum (2026-10-29):** SDK distribution, plan compatibility, LSP scope, and
the TypeScript runtime gap are recorded below for review. The original decision
is unchanged.

## Context

`eudkit` has two Frontends with different execution models:

- The Python Frontend executes the entry module with `runpy.run_path` and builds `HIR` in-process through PyO3. Module-level code can use `open()`, `json.load()`, `csv`, comprehensions, and ordinary Python libraries.
- The TypeScript Frontend parses source with `oxc` and lowers it statically to `HIR`. It is deterministic and does not require Node.js at build time.

Issue #56 proposed moving Python to the same strict parse-only model. That would give both Frontends a single static architecture, but it also has real costs:

- Build-time data loading (`open()`, `json.load()`, `csv`, project manifests) would need a separate codegen step or compiler-owned file built-ins.
- Standard expressions, comprehensions, and library transformations would be replaced by a documented subset.
- The compiler would grow a static evaluator and, eventually, a Rust Python parser.
- The authoring loop would lose behavior that already works well for Python mapmakers.

The single-binary argument is narrower than it first appears. Map authors already have Python or Node.js installed, because that is how they write scripts. The population that benefits from a host-runtime-free build is different:

- CI runners and minimal containers,
- headless build servers,
- distribution to people who only compile a checked-in artifact,
- LSP and IDE tooling that should not spawn a language runtime,
- builds that must not execute arbitrary pull-request code,
- reproducible builds.

The existing `HIR` seam already separates language-specific lowering from the Rust Backend. The missing piece is not a new IR or a live protocol; it is a canonical, frozen representation of `HIR`.

## Decision

1. **Keep host execution for Python authoring.** Python scripts continue to run with `runpy.run_path` and construct `HIR` in-process through PyO3. `open()`, `json.load()`, `csv`, comprehensions, and third-party build-time libraries remain available.
2. **Keep the static TypeScript Frontend.** TypeScript continues to parse and lower with `oxc`. It does not switch to Node.js execution.
3. **Serialize `HIR`, not a DSL AST.** Both Frontends emit a versioned **HIR Plan** with the `.eudhir` extension. The plan contains an `HIR` module plus a `Map Context` fingerprint. Language semantics stay in the Frontends; the Backend validates and lowers one representation.
4. **Compile plans without a host runtime.** `eudkit compile --plan <plan.eudhir>` reads a plan, validates every cross-reference, verifies the map fingerprint, and runs the normal `HIR` → `LIR` → Map pipeline without starting Python or Node.js.
5. **Provide an explicit synthesis step.** `eudkit synth` runs the selected Frontend and writes the plan. `eudkit build` remains the one-command authoring path and may use the same implementation internally.
6. **Make drift visible.** `eudkit synth --check` regenerates the plan in memory or in a temporary file and compares it with the committed artifact. CI can run this separately from `eudkit compile --plan`.
7. **Keep strict parse-only as an optional future mode.** If compiling Python source without a Python interpreter becomes a hard requirement, a parse-only Frontend can be added behind a flag. It is not the default and does not replace host execution.
8. **Do not change reactive semantics.** Reactive rules are serialized before their existing `HIR` lowering. ADR-0005 remains the timing authority, and #55 remains the single-source reactive lowering work.
9. **Distribute the SDKs independently.** The Python SDK is its own `eudkit`
   distribution, built with `maturin`; the TypeScript SDK is an npm tarball
   built with `npm pack`. The monolithic `eudplib` wheel stops shipping the
   `eudkit` package and declares it as a runtime dependency, because the
   deprecation shim re-exports `eudkit`. Both SDKs follow the `eudkit` release
   tag, not the `eudplib` version line.
10. **Make synthesis explicit in both languages.** `eudkit synth` and
    `python -m eudkit.synth` write a plan; `eudkit compile --plan` consumes it.
    `eudkit build` remains the one-command authoring path and may wrap both
    internally.
11. **Scope the language server to domain knowledge.** `eudkit-lsp` keeps
    Map Context symbols, canonical IDs, EUD offsets, and pre-compile
    diagnostics. It does not compete with Pyright, basedpyright, tsserver, or
    vtsls on generic hover, signature help, or type information.
12. **Do not claim host-executed TypeScript yet.** Static `oxc` remains the
    TypeScript default. A `--runtime=node` mode is an explicit, gated
    experiment; it must not silently change native control-flow semantics.

### Before

```text
Python source --runpy--> Python SDK --PyO3--> HIR --\
                                                       \
TypeScript source --oxc--> TS compiler ----------------> Rust Backend --> LIR --> SCX
                                                       /
                                          Map Context /
```

The Python path requires a Python interpreter and the PyO3 extension. The TypeScript path is already static. There is no reusable frozen artifact.

### After

```text
Authoring:
  Python source --runpy--> Python SDK --PyO3--> HIR --\
                                                        >-- HIR Plan (.eudhir)
  TypeScript source --oxc--> TS compiler -------------/          |
                                                                 |
CI / distribution:                                               v
  plan.eudhir --validate--> Rust Backend --> LIR --> SCX
                 (no Python, no Node.js)
```

`eudkit build` wraps synthesis and compilation for the authoring loop. `eudkit synth` and `eudkit compile --plan` expose the two halves explicitly.

## Plan envelope

The initial schema is JSON for reviewability and diffability:

```json
{
  "format": "eudkit-hir-plan",
  "schema_version": 1,
  "module": { "...": "canonical HirModule" },
  "map": {
    "fingerprint": "aed7da6034504d71",
    "name": "basemap.scx"
  }
}
```

Rules:

- `format` and `schema_version` are mandatory. Unknown versions are rejected with a diagnostic.
- `module` is the existing `HirModule`. There is no second IR tier and no Frontend-specific HIR.
- `map.fingerprint` is a stable hash of the input `scenario.chk`. It binds the plan to the `Map Context` used during synthesis. A mismatch is an error, not a warning.
- `map` may be absent only for programmatic plan construction; the CLI always writes it.
- Plans are data. The Backend validates IDs, block references, function references, byte buffers, function pointers, and expression references before indexing into the module. A malformed plan must produce a diagnostic, not a panic.
- JSON is the canonical review format. A compact or binary encoding can be added later without changing the semantic schema; size and compile-time measurements belong to the validation spike in #56.

### Compatibility policy

- `schema_version` is an integer. The current version is `1`.
- Unknown `format` values and unknown `schema_version` values are rejected with
  a diagnostic. A newer compiler must not guess at an older or newer layout.
- Within one schema version, only additive optional fields are allowed.
  Unknown fields are ignored on read; the compiler does not rewrite plans, so
  no data is silently lost in a build.
- A breaking change bumps `schema_version`. The compiler supports the current
  version for compilation and the previous version for one minor release with a
  deprecation warning. After that window, the previous version requires
  re-synthesis.
- `eudkit synth --check` always emits the current version. A plan that is
  accepted only for diagnostics is never silently upgraded in place.
- `map.fingerprint` is mandatory for CLI-produced plans. A missing fingerprint
  is accepted only for programmatic plans and produces a compile-time warning.
- The fingerprint is an identity check, not a security primitive. It is an
  FNV-1a hash of `scenario.chk`; a collision is not a trust boundary.

### Optional source metadata

Source metadata is not required to compile a plan, but it is needed to anchor
diagnostics back to the author's file. It is an additive optional field:

```json
{
  "source": {
    "entry": "main.py",
    "language": "python",
    "files": ["main.py", "balance.json"]
  }
}
```

Per-statement source spans are a later additive extension. Until they exist,
plan-based diagnostics are anchored at the plan file or a trigger label, not
at a source line. This limitation is why the LSP plan-based diagnostics in the
companion audit are phased behind source metadata.

## SDK distribution and tooling

The plan boundary is only useful if authors can produce plans without adopting
the compiler repository as their build system.

### Python

- Distribution: `eudkit`, import package `eudkit`, native module `eudkit._rust`.
- Artifacts: abi3 wheels for Linux x86_64/aarch64, macOS arm64/x86_64, and
  Windows x64; a `cp314t` wheel for free-threaded CPython 3.14; and an sdist.
- Build: `maturin`, with `eudkit-core` and `eudplib-stormlib` compiled in.
  Prebuilt wheels are the supported install path because a source build needs
  Rust, CMake, Ninja, zlib, bzip2, and StormLib.
- Types: `py.typed` plus complete `.pyi` stubs for the DSL, session, runner,
  synth entry point, and PyO3 surface. Stubs are checked with `stubtest`, mypy,
  and pyright.
- Synthesis: `python -m eudkit.synth --entry main.py --plan map.eudhir` runs
  the entry module with `runpy`, builds HIR in-process, and writes the plan.
  `open()`, `json.load()`, `csv`, comprehensions, and third-party libraries
  remain available.
- Ownership: `eudplib` stops including `eudkit` in its wheel, moves the
  `eudkit._rust` PyO3 build into this distribution, and declares `eudkit` as a
  runtime dependency because its shim re-exports it. The two distributions
  must never own the same package directory.

### TypeScript

- Distribution: `@eudkit/sdk`, ESM, `dist/` with `.d.ts` declarations.
- Artifact: `npm pack` tarball, installable from a local path or a GitHub
  Release asset. `main` and `types` point at `dist/`, never at `src/`.
- Synthesis: static `oxc` remains the default. `synth()` is a Node wrapper
  around `eudkit synth`; it is not a second compiler.
- Host operations: the static Frontend cannot execute `fs.readFileSync()`.
  A Node data pre-pass is a possible interim; full host execution needs the
  recording-transform spike described below.
- Release: `npm pack`, clean-install smoke test, `tsc --noEmit`, then
  `npm publish --provenance`.

### Release contract

- One `v<version>` tag drives the Rust workspace, both SDKs, and the editor
  packages. The existing version job is extended to check every manifest and
  lockfile.
- The release workflow builds wheels, the Python sdist, and the npm tarball
  alongside the binaries. Every asset gets a SHA-256 checksum.
- A clean-room job compiles a committed plan with the `eudkit` binary in a
  container that has no Python and no Node.js.
- Publishing uses the tested release assets, not a rebuild. PyPI uses trusted
  publishing; npm uses provenance and a scoped token.
- The standalone `eudkit` wheel is published before the `eudplib` shim that
  depends on it, so `pip install eudplib` never resolves a missing dependency.

## TypeScript host execution is not yet solved

The static Frontend parses TypeScript and lowers EUD constructs. Executing the
same module under Node is not equivalent:

- `if (deaths > 0)` would become a build-time branch instead of a trigger
  condition.
- `when(cond, cb)` would run `cb` once instead of recording a reactive rule.
- `for` loops would run in JavaScript instead of lowering to trigger loops or
  unrolled actions.

A recording transform could rewrite EUD regions into builder calls before
execution. That is a compiler frontend in JavaScript, not a small runtime
shim. Until a time-boxed spike proves that plans match the static Frontend on
representative fixtures, `--runtime=node` must remain an explicit experiment.
The static mode is fully supported, and a data-only Node pre-pass is the
preferred interim for file and JSON loading.

## Language server boundary

`eudkit-lsp` is a domain layer, not a general language server:

| Concern | Owner |
| --- | --- |
| Types, signatures, imports, refactors | Pyright/basedpyright, tsserver/vtsls |
| General diagnostics | Host language server |
| Map Context completion and validation | `eudkit-lsp` |
| Canonical unit/weapon IDs and constants | `eudkit-lsp` |
| EUD memory offset annotations | `eudkit-lsp` |
| Trigger-limit and VProc diagnostics | `eudkit-lsp`, backed by core diagnostics |
| Authoritative compile diagnostics | `eudkit` CLI; LSP mirrors a subset |

Symbol hover stays because it resolves map-defined names to IDs. Call hover
and signature help are removed because the host language server will have the
SDK stubs and declarations shipped by this plan. `signatureHelpProvider` is
not advertised. Removing these features is a deliberate, documented LSP
surface change, not an accident of the plan branch.

## Alternatives considered

| Alternative | Why it was not chosen as the default |
|---|---|
| Strict parse-only Python | Removes legitimate build-time data loading, requires a static evaluator and eventually a Rust Python parser, and does not improve the authoring loop. Remains an optional mode. |
| Host execution with live IPC | Adds process lifecycle, framing, version negotiation, and debugging complexity. A file artifact is simpler, cacheable, reviewable, and sufficient. |
| Sandboxed host execution | Improves determinism and security but is hard to make airtight and still requires the host runtime. A reasonable hardening step, not the architecture. |
| Embedded Python/JS runtime | Avoids a host dependency but has incomplete compatibility, no third-party packages, and significant size and maintenance costs. |
| Macro expansion / codegen to a manifest | Equivalent to the plan model with an extra user-facing step. The plan is the generated artifact. |
| Parse-only with controlled file built-ins | Preserves some data-loading DX and determinism, but is still a static evaluator and still excludes arbitrary libraries. Useful as an optional untrusted-build mode. |

## Consequences

- **DX:** Python authors keep the full language for build-time work. TypeScript authors keep the static `oxc` model.
- **Determinism:** the frozen artifact is deterministic. Regenerating it on every CI run is only as deterministic as the host code; CI should compile the committed plan and run `synth --check` separately.
- **Distribution:** a checked-in plan can be compiled by the Rust binary alone.
- **Security:** CI can compile a plan without executing Frontend code. The plan is still data and must be validated.
- **Cross-language consistency:** both Frontends converge at `HirModule`. Conformance tests can compare canonical plans instead of debug text.
- **Maintenance:** no static Python evaluator and no Rust Python parser. The new surface is a schema, serialization, validation, and two CLI commands.
- **Migration:** existing `eudkit build` behavior remains valid. Plans are additive; projects can adopt them when they need deterministic CI or host-runtime-free distribution.
- **Packaging:** `eudkit` becomes its own Python distribution and `@eudkit/sdk`
  becomes a real npm package. `eudplib` stops shipping `eudkit` and declares it
  as a runtime dependency, because its shim imports `eudkit` and
  `eudkit._rust` at import time. This is a documented install-path change, not
  a runtime API change.
- **Tooling:** the LSP loses call hover and signature help. Symbol hover and
  Map Context diagnostics remain, and trigger/VProc diagnostics move into a
  shared core diagnostics path so the CLI and LSP agree.
- **TypeScript gap:** host-executed TypeScript is explicitly unresolved. Static
  `oxc` ships; `--runtime=node` is gated by the spike and must not become a
  silent default.
- **Release:** wheels, sdist, and the npm tarball are first-class release
  assets with checksums and clean-install tests. A plan can be compiled by a
  single binary with no host runtime.

## Validation

Issue #56 validates this decision with a representative fixture:

1. A Python fixture that uses `json.load()` or `csv` produces a plan.
2. A TypeScript fixture produces the same schema.
3. `eudkit compile --plan` runs without Python or Node.js.
4. The plan compiles to the same map bytes as the source path.
5. `eudkit synth --check` detects drift.
6. Plan size and compile time are measured.
7. A parse-only prototype of the same fixture is compared on effort and compatibility.
8. A wheel install and an sdist build both produce a working `eudkit` package.
9. An `npm pack` tarball installs, type-checks, and runs `synth()`.
10. The LSP protocol tests assert the domain-only surface: symbol hover and
    diagnostics are present, signature help is absent.

If validation fails, this ADR is amended or superseded before the plan format is declared stable.

## References

- ADR-0001: Two-tier IR in Rust.
- ADR-0003: TypeScript Frontend and migration CLI.
- ADR-0005: Trigger Loop timing model.
- #54: Hover and signature help.
- #55: Lower reactive rules once across Python and TypeScript Frontends.
- #56: Frontend execution model decision and spike.
- `docs/architecture/cdk-synthesis-tooling-plan.md`: SDK, CLI, and release plan.
- `docs/architecture/lsp-audit.md` (on `feat/lsp-mvp`): LSP retain/prune audit
  and domain roadmap.
