# Host-Executed Frontends with Frozen HIR Plans

**Status:** Accepted for implementation; validation is tracked in #56.

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

## Validation

Issue #56 validates this decision with a representative fixture:

1. A Python fixture that uses `json.load()` or `csv` produces a plan.
2. A TypeScript fixture produces the same schema.
3. `eudkit compile --plan` runs without Python or Node.js.
4. The plan compiles to the same map bytes as the source path.
5. `eudkit synth --check` detects drift.
6. Plan size and compile time are measured.
7. A parse-only prototype of the same fixture is compared on effort and compatibility.

If validation fails, this ADR is amended or superseded before the plan format is declared stable.

## References

- ADR-0001: Two-tier IR in Rust.
- ADR-0003: TypeScript Frontend and migration CLI.
- ADR-0005: Trigger Loop timing model.
- #55: Lower reactive rules once across Python and TypeScript Frontends.
- #56: Frontend execution model decision and spike.
