# CDK-Style Synthesis and Decoupled SDK Tooling

**Status:** Proposed for implementation
**Issue:** #56
**Decision record:** [ADR-0006](../adr/0006-frozen-hir-plans.md)
**Companion audit:** `docs/architecture/lsp-audit.md` (on `feat/lsp-mvp`)

## 1. Context

Issue #56 selected a CDK-style boundary for `eudkit`:

- Python keeps host execution during authoring.
- TypeScript keeps the static `oxc` Frontend.
- Both Frontends emit a canonical, versioned HIR Plan (`.eudhir`).
- The Rust Backend compiles a plan without Python or Node.js.
- CI compiles the committed plan and separately checks for drift.

The plan boundary already landed on the `eudkit` branch in `4ae8b9c1`:

- `HirPlan` envelope with format and schema checks.
- Map Context fingerprint bound to the input `scenario.chk`.
- Cross-reference validation for functions, blocks, variables, byte buffers,
  function pointers, and expressions.
- `eudkit synth`, `eudkit compile --plan`, and `eudkit synth --check`.
- Python `Session.dump_plan()` / `write_plan()` and `eudkit.runner --plan`.
- Byte-identical plan-vs-source build tests and drift tests.

This document is the delivery plan for the remaining architectural work:

1. Standalone Python and TypeScript authoring SDK distributions.
2. Release CI for `maturin` wheels, the Python source tarball, and `npm pack`.
3. A single CLI contract for synthesis and host-runtime-free compilation.
4. A bounded decision process for host-executed TypeScript.
5. The handoff to the LSP scope audit.

## 2. Goals and non-goals

### Goals

- `pip install eudkit` provides the Python DSL, the PyO3 runtime, complete type
  information, and `python -m eudkit.synth`.
- `npm install @eudkit/sdk` provides the TypeScript DSL, declarations, and a
  `synth()` entry point that can be called from Node.js.
- The Python SDK can synthesize a plan while normal host-language code runs
  during authoring. The TypeScript SDK synthesizes statically; host-executed
  TypeScript is gated by section 6.4.
- `eudkit compile --plan` runs with no Python and no Node.js on `PATH`.
- Release artifacts are built once, smoke-tested from a clean install, and
  attached to the same GitHub Release as the `eudkit` binaries.
- The LSP keeps only domain features that host language servers cannot provide.

### Non-goals

- Replacing ADR-0005 or changing reactive semantics.
- Removing the Python package, PyO3 bindings, or Python runtime API.
- A live IPC protocol between the compiler and a host runtime.
- A new DSL, a new IR tier, or a new map output format.
- Supporting legacy `eudplib`/`epScript` workflows.
- Making `.py` source compile without Python. Only frozen plans must do that.
- Full host-executed TypeScript before the spike in section 6.4 passes.

## 3. Target architecture

```text
Authoring                         Synthesis                     Build
─────────────────────────────────────────────────────────────────────────
Python source ──runpy──────────► HIR ─┐
                                      ├──► HIR Plan (.eudhir) ──► Rust Backend
TypeScript source ──oxc────────► HIR ─┘        │                  │
                                                │                  ▼
TypeScript + Node ──recording──► HIR ──────────┘               LIR → SCX
       (proposed, gated)                          (no Python, no Node)
```

`eudkit build` remains the one-command authoring path. It may synthesize and
compile internally. `eudkit synth` and `eudkit compile --plan` expose the two
halves so CI and distribution can use only the second half.

### 3.1 Artifact boundary

The plan is data, not executable code:

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

- `format` and `schema_version` are mandatory.
- `module` is the existing `HirModule`; there is no second HIR schema.
- `map.fingerprint` binds the plan to the Map Context used during synthesis.
  A mismatch is an error, not a warning.
- A plan without `map` is accepted only for programmatic construction; the CLI
  always writes it.
- A malformed plan must produce a diagnostic, never a panic.
- JSON is the canonical review format. A compact or binary encoding is a later
  decision backed by measurements, not a schema change.

### 3.2 Runtime matrix

| Frontend | Authoring runtime | Synthesis | Compile runtime |
| --- | --- | --- | --- |
| Python | CPython + PyO3 | `runpy` + in-process HIR | Rust only |
| TypeScript (default) | none | `oxc` in Rust | Rust only |
| TypeScript (`--runtime=node`, proposed) | Node.js + SDK | Node executes and records HIR | Rust only |

The Python row is the accepted default. The TypeScript static row is the
accepted default. The Node row is a gated experiment; see section 6.

## 4. Repository and package layout

Target layout:

```text
packages/
  python-sdk/
    pyproject.toml
    README.md
    src/eudkit/
      __init__.py
      __init__.pyi
      __main__.py
      _rust.pyi
      ast_lowering.py
      ast_lowering.pyi
      cli.py
      runner.py
      runner.pyi
      session.py
      session.pyi
      synth.py
      synth.pyi
      types.py
      types.pyi
      py.typed
    tests/
  ts-sdk/
    package.json
    package-lock.json
    tsconfig.json
    tsconfig.build.json
    src/
      index.ts
      synth.ts
      plan.ts
    scripts/
      check-docs.mjs
      check-pack.mjs
    tests/
src/rust/
  eudkit-core/        HIR, plan envelope, validation, diagnostics
  eudkit-cli/         synth, compile --plan, build, inspect
  eudkit-ts/          oxc Frontend
  eudkit-lsp/         domain language server
```

Current locations and moves:

| Current | Target | Reason |
| --- | --- | --- |
| `src/eudkit/` | `packages/python-sdk/src/eudkit/` | Separate Python distribution and build context |
| root `[tool.maturin]` | `packages/python-sdk/pyproject.toml` | The native extension must move with the `eudkit` package; `eudplib` keeps its own pyproject without the `eudkit` extension |
| root `build_backend.py` | split: SDK build config; `eudplib` keeps locale/epScript packaging | The PyO3 extension build moves with `eudkit` |
| `packages/ts-sdk/` | stays | Already a separate package |
| `packages/ts-sdk/package.json` name `eudkit` | `@eudkit/sdk` | Avoid an unscoped npm name collision and signal the SDK scope |

The Rust workspace stays at `src/rust`. The Python SDK `pyproject.toml` points
its `manifest-path` at the workspace root. If maturin rejects an out-of-tree
manifest during sdist construction, add a thin
`packages/python-sdk/rust/Cargo.toml` cdylib crate that depends on
`eudkit-core`, and move `eudkit_py.rs` plus the `#[pymodule]` wrapper into it;
do not duplicate the bindings.

## 5. Python SDK

### 5.1 Distribution

| Item | Decision |
| --- | --- |
| Distribution name | `eudkit` |
| Import package | `eudkit` |
| Native module | `eudkit._rust` |
| Build backend | `maturin` |
| ABI | `abi3-py310` for GIL builds; separate `cp314t` wheel for free-threaded 3.14 |
| Wheels | Linux x86_64/aarch64, macOS arm64/x86_64, Windows x64 |
| Source distribution | `eudkit-<version>.tar.gz` (canonical PyPI name) |
| Build requirements | Rust, CMake, Ninja, zlib, bzip2, StormLib sources |

`eudkit-core` depends on `eudplib-stormlib`, so a source build is not a
pure-Python install. Prebuilt wheels are the supported path. The sdist is for
platforms with a working C/C++ toolchain and must be tested in a clean
container, not assumed to work.

Build commands (run from `packages/python-sdk`):

```bash
maturin build --release --locked --out dist --features pyo3/abi3-py310
maturin sdist --out dist
```

`maturin sdist` writes `eudkit-<version>.tar.gz`. Do not rename it: PyPI
requires the sdist filename to match the distribution name and version.

Linux wheels are repaired to `manylinux_2_28`. macOS wheels declare the
minimum supported macOS version. Windows wheels are `win_amd64`.

### 5.2 Type stubs

The SDK ships `py.typed` and complete stubs for every public surface:

- `eudkit/__init__.pyi`: all DSL exports, `eud`, `fn`, `func_ptr`, constants.
- `eudkit/types.pyi`: `Expr`, `Var`, `MaskedVar`, `Condition`, `ActionSpec`,
  `Array`, `VArray`, `CUnit`, `CSprite`, `MapString`, `StringBuffer`, and the
  stock condition/action helpers.
- `eudkit/session.pyi`: `Session`, `MapContext`, `AmbientSessionProxy`, and
  plan methods.
- `eudkit/ast_lowering.pyi`: `EudFunction`, `EudFuncPtr`, `LoopRange`.
- `eudkit/runner.pyi` and `eudkit/synth.pyi`: module entry points.
- `eudkit/_rust.pyi`: the PyO3 `RawSession`, `CompilationReport`, and errors.

Stubs are checked in CI with:

```bash
python -m mypy.stubtest eudkit
python -m mypy --strict packages/python-sdk/src/eudkit
python -m pyright packages/python-sdk/src/eudkit
```

The stubs are the source of truth for editor completion and hover. The LSP
must not duplicate them; it only adds Map Context and EUD-specific knowledge.

### 5.3 `python -m eudkit.synth`

New module `eudkit.synth`:

```text
python -m eudkit.synth --entry main.py --plan build/main.eudhir \
  [--input basemap.scx] [--config eudkit.toml] [--check] [--quiet] [--json]
```

Behavior:

1. Resolve `--config` or the nearest `eudkit.toml`.
2. Resolve the input map from `--input` or `build.input_map`.
3. Create a `Session`, set the entry directory on `sys.path`, and execute the
   entry module with `runpy.run_path(..., run_name="__main__")`.
4. Call `Session.write_plan()` or `Session.dump_plan()`.
5. With `--check`, compare the generated plan with the existing plan and exit
   nonzero on drift. Do not compile a map.

Flags:

| Flag | Required | Meaning |
| --- | --- | --- |
| `--entry`, `-e` | yes, unless config | Python entry module |
| `--plan`, `-p` | yes | Output or checked `.eudhir` path |
| `--input`, `-i` | yes, unless config | Base `.scx`/`.scm` for Map Context |
| `--config`, `-c` | no | `eudkit.toml` path |
| `--check` | no | Compare instead of write; nonzero on drift |
| `--quiet` | no | Suppress the success line |
| `--json` | no | Machine-readable diagnostics on stderr/stdout |

The module is a thin wrapper around the same Python path as `eudkit build`. It
does not reimplement compilation and does not invoke the Rust CLI.

### 5.4 Host execution guarantee

The Python Frontend is not parse-only. This must keep working:

```python
import csv
import json
from pathlib import Path
from eudkit import *

data = json.loads(Path("balance.json").read_text())
rows = list(csv.DictReader(Path("waves.csv").open()))
names = [row["unit"] for row in rows if int(row["wave"]) > data["start_wave"]]

for name in names:
    when(Deaths(Player1, AtLeast, 1, name), lambda: KillUnit(name, Player1))
```

`open()`, `json.load()`, comprehensions, third-party packages, and build-time
code generation are supported because the entry module runs in CPython. The
plan freezes only the resulting HIR.

### 5.5 Packaging migration

The current monolithic `eudplib` wheel includes both `eudplib` and `eudkit`.
That cannot coexist with a standalone `eudkit` wheel: two distributions would
own `eudkit/`, and uninstalling one could remove the other.

Migration:

1. Move the root `[tool.maturin]` section into
   `packages/python-sdk/pyproject.toml`, including
   `module-name = "eudkit._rust"`, `manifest-path`, `python-source`, features,
   and `include`. Removing `"eudkit"` from `python-packages` alone is not
   enough: the native extension would still install into `eudkit/` and collide
   with the standalone wheel.
2. Move the Python sources into `packages/python-sdk/src/eudkit`.
3. Make the remaining `eudplib` wheel stop building `eudkit._rust` and declare
   `eudkit` as a runtime dependency. The current shim executes `import eudkit`
   and `from eudkit import _rust` at import time, so an optional extra would
   leave `import eudplib` broken. If `eudplib` must remain installable without
   `eudkit`, change the shim to a lazy import with a clear error; treat that as
   a separate API decision.
4. Replace the `eudplib` maturin backend with a Python packaging backend (or a
   custom backend that packages the shim and any legacy epScript C library).
   `eudplib` is not automatically "pure Python": legacy epScript still loads
   `libepScriptLib`, so keep that build if the legacy surface remains. What
   must move is the `eudkit._rust` PyO3 extension.
5. Update `publish.yml`. It currently builds the PyO3 extension from the root
   `pyproject.toml`; after the split it must publish only the `eudplib` shim,
   or be retired. It must never build `eudkit._rust`.
6. Add a release note: users who relied on `eudplib` to provide `eudkit` now get
   it as a dependency, and direct users must `pip install eudkit`.
7. Keep the `eudplib` version line independent (`0.81.x`), while `eudkit`
   follows the Rust workspace version (`0.1.x`).

The old wheel must stop shipping `eudkit` in the same release that first
publishes the standalone wheel. There is no safe overlap window. Publish the
standalone `eudkit` wheel before the `eudplib` shim that depends on it;
otherwise `pip install eudplib` resolves to a missing dependency.

## 6. TypeScript SDK

### 6.1 Package and build

Target `package.json`:

```json
{
  "name": "@eudkit/sdk",
  "version": "0.1.2",
  "type": "module",
  "main": "./dist/index.js",
  "types": "./dist/index.d.ts",
  "exports": {
    ".": {
      "types": "./dist/index.d.ts",
      "import": "./dist/index.js"
    }
  },
  "files": ["dist", "README.md", "LICENSE"],
  "sideEffects": false,
  "engines": { "node": ">=18" },
  "scripts": {
    "build": "tsc -p tsconfig.build.json",
    "check-docs": "node scripts/check-docs.mjs",
    "check-pack": "node scripts/check-pack.mjs",
    "prepack": "npm run build",
    "test": "npm run check-docs && npm run build && npm run check-pack"
  }
}
```

`tsconfig.build.json` emits ESM, declarations, and source maps to `dist/`. The
package must not point `main`/`types` at `src/index.ts`; that only works inside
the repository and produces a tarball that consumers cannot type-check.

### 6.2 Static `oxc` mode

Static mode is the default and the only mode that is release-ready:

```bash
eudkit synth --input basemap.scx --entry main.ts --plan build/main.eudhir
eudkit synth --runtime=static --input basemap.scx --entry main.ts \
  --plan build/main.eudhir
```

The Rust Frontend parses TypeScript/JavaScript with `oxc`, lowers it to HIR,
and writes the plan. No Node.js is required at synthesis or compile time.

### 6.3 Host operations in TypeScript

The static Frontend cannot execute `fs.readFileSync()`, `JSON.parse()` of a
runtime file, or a third-party utility. The source is parsed, not run. Native
`if`/`for`/`while` in the source are also EUD control flow, not build-time
control flow.

There are three viable designs:

| Design | Host operations | EUD semantics | Cost | Decision |
| --- | --- | --- | --- | --- |
| A. Static `oxc` only | none at synthesis | existing static semantics | none | Default |
| B. Data pre-pass | top-level data modules can read files | static compiler consumes exported data | medium | Candidate interim |
| C. Node recording transform | full Node execution | rewrite EUD control flow into builder calls | high | Spike only |

Design C is the only way to claim full host-language parity. Plain Node
execution is not enough: `if (deaths > 0)` would be evaluated by Node at build
time instead of becoming a trigger condition, and `when(cond, cb)` would run
`cb` once instead of recording a reactive rule. A transform must rewrite the
EUD regions before execution, or the SDK must expose an explicit builder API
instead of native control flow. Both are significant language-surface changes.

Design B is a pragmatic interim: a designated `data.ts` module runs under Node,
exports JSON-serializable values, and the static compiler resolves those values
through a generated data module. It satisfies file/JSON loading for data but
does not run arbitrary host code inside EUD callbacks.

**Do not market the TypeScript SDK as host-executed until design C passes the
spike in section 6.4.** The static mode remains fully supported.

### 6.4 TypeScript runtime spike (go/no-go)

Scope: one week, time-boxed, on a branch; no release commitment.

Candidate implementation:

1. Use the TypeScript compiler API or Babel to transform only EUD regions:
   `when`, `until`, `after`, `every`, `once`, `fn` callbacks, and top-level
   action/condition calls.
2. Rewrite native `if`/`for`/`while` inside those regions into builder calls
   that emit HIR instead of executing host branches.
3. Execute the transformed module under Node with a recording runtime.
4. Serialize the recorded HIR through the same `HirPlan` envelope.
5. Compare the plan with the static `oxc` plan on the conformance fixtures.

Go criteria:

- 20 representative fixtures produce byte-identical plans or documented,
  intentional differences.
- A fixture using `fs.readFileSync()` and a third-party utility produces a valid
  plan.
- Native `if` inside a `when` callback does not become a build-time branch.
- Error locations map back to the original TypeScript source.
- No second HIR schema and no compiler-owned JavaScript runtime.

No-go criteria:

- The transform cannot distinguish host data flow from EUD control flow
  reliably.
- Plans differ from static mode in ways that change map behavior.
- The maintenance surface approaches a second TypeScript compiler.
- Build time or memory exceeds the agreed budget.

If no-go: ship static mode, document the limitation, and keep design B as the
data-loading escape hatch. If go: expose `--runtime=node` as an explicit opt-in
and keep `--runtime=static` as the default.

### 6.5 `synth()` runtime API

The first release exposes a Node wrapper around the Rust CLI:

```ts
export interface SynthOptions {
  input: string;
  entry: string;
  plan: string;
  binary?: string;   // defaults to EUDKIT_BIN or "eudkit" on PATH
  check?: boolean;
}

export interface SynthResult {
  plan: string;
  stdout: string;
  stderr: string;
}

export function synth(options: SynthOptions): Promise<SynthResult>;
```

It spawns `eudkit synth` with the same flags as the CLI. It does not parse
TypeScript in JavaScript. This is a convenience API, not a second compiler.

A future `runtime: "node"` option is added only if section 6.4 passes:

```ts
await synth({ runtime: "node", input, entry, plan });
```

### 6.6 `npm pack` contents

`npm pack` must produce a tarball that works outside the repository:

```text
package/
  dist/index.js
  dist/index.d.ts
  dist/index.js.map
  dist/synth.js
  dist/synth.d.ts
  README.md
  LICENSE
  package.json
```

`scripts/check-pack.mjs` runs `npm pack --dry-run --json` and fails if `src/`,
tests, or build scripts leak into the tarball, or if `dist/index.d.ts` is
missing. CI additionally installs the tarball into a temporary project and
runs `tsc --noEmit` against a file that imports the SDK.

## 7. CLI contract

### 7.1 `eudkit synth`

```text
eudkit synth [--config eudkit.toml]
             [--input basemap.scx]
             [--entry main.py|main.ts|main.js]
             [--plan out.eudhir]
             [--check]
             [--runtime auto|python|static|node]
             [--json] [--quiet]
```

| Flag | Values | Default | Notes |
| --- | --- | --- | --- |
| `--config`, `-c` | path | nearest `eudkit.toml` | Project defaults |
| `--input`, `-i` | path | `build.input_map` | Required for fingerprinting |
| `--entry`, `-e` | path | `build.entry` | Extension selects Frontend |
| `--plan`, `-p`, `--output`, `-o` | path | `output_map.with_extension("eudhir")`, else `plan.eudhir` | Output or checked path |
| `--check` | flag | off | Compare, do not write |
| `--runtime` | `auto`, `python`, `static`, `node` | `auto` | `auto` = Python for `.py`, static for `.ts`/`.js` |
| `--json` | flag | off | Machine-readable diagnostics |
| `--quiet` | flag | off | Suppress success output |

Behavior:

- `auto` must never silently execute Node for a `.ts` file. `node` is explicit.
- `python` on a `.ts` entry is a usage error.
- `static` on a `.py` entry is a usage error until a parse-only Python Frontend
  exists.
- `--check` exits `1` on drift and prints the first differing plan path.
- The Map Context fingerprint is always written for CLI-produced plans.

### 7.2 `eudkit compile --plan`

```text
eudkit compile --plan build/main.eudhir
                --input basemap.scx
                --output out.scx
                [--config eudkit.toml]
                [--obscurate] [--seed N]
                [--json]
```

| Flag | Required | Notes |
| --- | --- | --- |
| `--plan`, `-p` | yes | `.eudhir` artifact |
| `--input`, `-i` | yes, unless config | Must match the plan fingerprint |
| `--output`, `-o` | yes, unless config | Output map |
| `--config`, `-c` | no | Project defaults |
| `--obscurate` | no | Existing obscuration pass |
| `--seed` | no | Deterministic obscuration seed |
| `--json` | no | Machine-readable report |

No Python, Node.js, PyO3, or `eudkit` package is consulted. The command runs
from the `eudkit` binary alone.

### 7.3 `eudkit build`

`build` remains the one-command path:

```text
eudkit build --input basemap.scx --entry main.py --output out.scx
eudkit build --input basemap.scx --entry main.ts --output out.scx
eudkit build --input basemap.scx --entry main.ts --output out.scx \
  --plan-out build/main.eudhir
```

`--plan-out` is additive. `build` may use `synth` + `compile --plan` internally,
but its user-facing behavior and output report stay compatible.

### 7.4 Exit codes and diagnostics

| Code | Meaning |
| --- | --- |
| 0 | Success, including `--check` with no drift |
| 1 | Synthesis, validation, fingerprint, or compile error |
| 2 | Usage or configuration error |

`--json` emits one diagnostic object per line on stdout:

```json
{"severity":"error","code":"EUD-PLAN-001","message":"...","path":"...","line":12}
```

Human-readable diagnostics stay on stderr. The Rust CLI and the Python
`eudkit.synth` module use the same codes so editors and CI can parse both.

### 7.5 Examples

Python authoring, then a runtime-free CI compile:

```bash
python -m eudkit.synth --config eudkit.toml --entry main.py \
  --plan build/main.eudhir
eudkit compile --plan build/main.eudhir --input basemap.scx --output out.scx
```

TypeScript static authoring:

```bash
eudkit synth --input basemap.scx --entry main.ts --plan build/main.eudhir
eudkit compile --plan build/main.eudhir --input basemap.scx --output out.scx
```

Drift check in CI:

```bash
eudkit synth --check --config eudkit.toml --entry main.py \
  --plan build/main.eudhir
```

## 8. Release CI

### 8.1 Workflow topology

`release-eudkit.yml` keeps ownership of the tag, version checks, binaries, LSP
binaries, editor packages, and the GitHub Release. Add four jobs:

| Job | Purpose |
| --- | --- |
| `python-sdk-wheels` | Build and repair `eudkit` wheels |
| `python-sdk-sdist` | Build and test the Python source tarball |
| `ts-sdk-pack` | Build, pack, and clean-install the npm tarball |
| `sdk-smoke` | Install both SDKs and run plan/compile fixtures |

`publish.yml` currently builds the PyO3 extension from the root
`pyproject.toml`. After the split it must publish only the `eudplib` shim
(which depends on `eudkit`) or be retired; it must not build `eudkit._rust`.
Add `publish-eudkit-sdk.yml` or a publish job that runs on
`release: published`, downloads the verified SDK artifacts from the release,
and publishes them. Publishing from the tested artifacts avoids rebuild drift.

### 8.2 Python wheel matrix

| OS | Runner | Target | Compatibility |
| --- | --- | --- | --- |
| Linux x86_64 | `ubuntu-latest` | `x86_64-unknown-linux-gnu` | `manylinux_2_28_x86_64` |
| Linux aarch64 | `ubuntu-24.04-arm` | `aarch64-unknown-linux-gnu` | `manylinux_2_28_aarch64` |
| macOS arm64 | `macos-latest` | `aarch64-apple-darwin` | macOS minimum |
| macOS x86_64 | Intel macOS runner (e.g. `macos-15-intel`) or cross | `x86_64-apple-darwin` | macOS minimum |
| Windows x64 | `windows-latest` | `x86_64-pc-windows-msvc` | `win_amd64` |

Build two ABI families:

- `cp310-abi3` for GIL-enabled CPython 3.10+.
- `cp314t` for free-threaded CPython 3.14+.

The cp311-abi3 variant currently built by the root `eudplib` project is
optional for `eudkit`; add it only if a PyO3 API requires 3.11. Fewer wheels
is better.

Per-wheel steps:

```bash
pip install maturin twine
pip install auditwheel                       # Linux
maturin build --release --locked --out dist --features pyo3/abi3-py310
auditwheel repair dist/*.whl -w dist/       # Linux
rm dist/*-linux_*.whl                       # Linux: keep only the repaired wheel
twine check dist/*.whl
```

Upload each wheel as an artifact. Do not publish from a matrix job; collect and
verify first.

### 8.3 Python sdist

```bash
maturin sdist --out dist
```

Then, in a clean container with CMake, Ninja, zlib, bzip2, and a Rust
toolchain:

```bash
python -m pip install dist/eudkit-[0-9]*.tar.gz
python -c "import eudkit, importlib.metadata as m; print(m.version('eudkit'))"
```

The sdist test is mandatory even though wheels are the primary artifact. It
catches missing Rust sources, missing `Cargo.lock`, missing submodule sources
(StormLib), and `include`/`exclude` mistakes in `pyproject.toml`.

### 8.4 TypeScript tarball

```bash
npm ci
npm run build
npm pack
```

Then:

```bash
mkdir -p /tmp/eudkit-sdk-smoke && cd /tmp/eudkit-sdk-smoke
npm init -y
npm install /path/to/eudkit-sdk-<version>.tgz
node --input-type=module -e "import('@eudkit/sdk').then(m => console.log(typeof m.synth))"
npx tsc --noEmit --strict --module nodenext --moduleResolution nodenext smoke.ts
```

`smoke.ts` imports `synth`, `KillUnit`, `Player1`, and `SetTo` to prove both
runtime and declaration resolution.

Publishing:

```bash
npm publish eudkit-sdk-<version>.tgz --access public --provenance
```

Use an npm granular token scoped to `@eudkit`, or npm trusted publishing if the
repository is configured for it. Do not publish from a pull request.

### 8.5 Artifact naming and checksums

| Artifact | Example |
| --- | --- |
| Python wheel | `eudkit-0.1.2-cp310-abi3-manylinux_2_28_x86_64.whl` |
| Python sdist | `eudkit-0.1.2.tar.gz` |
| npm tarball | `eudkit-sdk-0.1.2.tgz` |
| CLI binary | `eudkit-linux-x86_64` |
| LSP binary | `eudkit-lsp-linux-x86_64` |
| Editor package | `eudkit-vscode-0.1.2.vsix` |
| Zed package | `eudkit-zed-0.1.2.tar.gz` |

Naming rules:

- Keep the canonical PyPI sdist name `eudkit-<version>.tar.gz`. Renaming it to
  `eudkit-python-<version>.tar.gz` would break PyPI filename/metadata rules.
  The Zed package is already prefixed `eudkit-zed-`, so there is no collision.
- Keep the npm tarball name `eudkit-sdk-<version>.tgz` (npm derives it from
  `@eudkit/sdk`) so it is distinguishable from the Python distribution.
- Generate a `.sha256` for every release asset, including wheels and tarballs.
- Extend `BINARY-SIZES.txt` to include SDK artifacts and their uncompressed
  install size.

### 8.6 Version synchronization

The release tag drives all `eudkit` versions:

| File | Field |
| --- | --- |
| `src/rust/Cargo.toml` | `[workspace.package].version` |
| `src/rust/Cargo.lock` | all `eudkit-*` packages |
| `packages/python-sdk/pyproject.toml` | `[project].version` |
| `packages/ts-sdk/package.json` | `version` |
| `packages/ts-sdk/package-lock.json` | root `version` |
| `editors/vscode/package.json` | `version` |
| `editors/zed/Cargo.toml` and `extension.toml` | `version` |
| `editors/zed/Cargo.lock` | `eudkit-*` package versions |

Extend the existing `version` job checks. A tag that disagrees with any package
must fail before any build starts.

## 9. Test and drift gates

| Gate | Command / fixture | Blocks release |
| --- | --- | --- |
| Plan schema | `cargo test -p eudkit-core plan` | yes |
| Plan validation | malformed references, bad IDs, unknown version | yes |
| Fingerprint | wrong input map is rejected | yes |
| Python host data | `tests/fixtures/plan_host_data/main.py` uses `json.load`/`csv` | yes |
| TypeScript static | equivalent `main.ts` produces the same schema | yes |
| Plan vs source | same map bytes from source build and plan compile | yes |
| Drift | `eudkit synth --check` fails after a source edit | yes |
| Runtime-free compile | `debian:bookworm-slim` + `eudkit` binary, no Python/Node | yes |
| Python wheel | clean venv install, `import eudkit`, synth fixture | yes |
| Python sdist | clean container build and install | yes |
| Stubs | `mypy.stubtest`, `mypy --strict`, `pyright` | yes |
| npm tarball | clean install, `tsc --noEmit`, `synth()` smoke | yes |
| LSP protocol | completion, symbol hover, diagnostics; no signature help | yes |
| LSP domain | map context inspection, unit/weapon/constant IDs, offsets, trigger/VProc diagnostics | yes |

The plan-vs-source byte identity test is the strongest regression gate. It must
run for both Frontends and for reactive rules covered by #55.

## 10. Phased delivery

| Phase | Work | Exit criteria |
| --- | --- | --- |
| 0 | ADR-0006 addendum and this plan | Reviewed; no code changes |
| 1 | Extract `packages/python-sdk`; move `eudkit` and its native build out of `eudplib`; add the dependency | `pip install eudkit` works; `pip install eudplib` imports and pulls `eudkit` |
| 2 | Add stubs and `python -m eudkit.synth` | `stubtest` passes; Python fixture writes a plan |
| 3 | Build and pack the TypeScript SDK | `npm pack` tarball installs and type-checks |
| 4 | Add `synth()` static wrapper and CLI `--runtime` | `synth()` writes a plan; no Node at compile |
| 5 | Release CI for wheels, sdist, npm tarball | Clean-install smoke jobs pass on all platforms |
| 6 | TypeScript runtime spike | Go/no-go recorded in ADR-0006 |
| 7 | LSP prune and domain additions | Audit acceptance criteria pass |
| 8 | Migration notes and release | Users have a single install path and a documented breaking change |

Phases 1–4 can proceed in parallel with the preparation of phase 7, but the
LSP prune ships only after the SDK stubs and declarations from phases 2–3 are
available. Phase 8 ships only after the wheel ownership migration and the LSP
surface change are both documented.

## 11. Risks and mitigations

| Risk | Impact | Mitigation |
| --- | --- | --- |
| `eudplib` and `eudkit` both ship `eudkit/` | Broken installs and uninstalls | Move the package and the `eudkit._rust` build out of `eudplib` in the same release; no overlap |
| Python sdist fails without CMake/StormLib | Users cannot build from source | Prebuilt wheels are primary; test sdist in a toolchain container |
| TypeScript host execution changes semantics | Silent map behavior changes | Keep static default; gate `--runtime=node` behind the spike |
| Plan schema drift | Old plans fail or compile incorrectly | Version check, additive fields, compatibility window |
| Plan has no source metadata | LSP diagnostics cannot point at source | Add optional `source` metadata before source-anchored diagnostics |
| LSP duplicates host-language features | Maintenance and user confusion | Keep only domain features; link to Pyright/tsserver |
| LSP lowering is expensive | Editor lag | Cache by plan mtime/fingerprint; run on open/save, not every keystroke |
| npm scope/name unavailable | Release blocked | Verify `@eudkit/sdk` and PyPI `eudkit` before phase 3 |
| CI matrix cost | Slow releases | abi3 wheels; cache Rust/Python/npm; build only on tags |

## 12. Open questions

1. Is the PyPI name `eudkit` available, or is `eudkit-sdk` required?
2. Is the npm scope `@eudkit` available, or should the package be
   `@eudplib/eudkit`?
3. Should `eudkit build` write a plan by default, or only with `--plan-out`?
4. Should plans be committed for every project, or only projects that need
   runtime-free CI?
5. Is the one-minor-release `schema_version` compatibility window in
   ADR-0006 sufficient, or should it be longer?
6. Does the TypeScript runtime spike pass, and does `--runtime=node` ship in
   the same release or a later one?
7. What plan-size and compile-time budgets are acceptable after the first
   measurements?

## 13. Definition of done

- [ ] `pip install eudkit` works on all supported platforms from a wheel.
- [ ] `pip install eudkit-<version>.tar.gz` works in a clean toolchain.
- [ ] `npm install eudkit-sdk-<version>.tgz` type-checks and runs `synth()`.
- [ ] `python -m eudkit.synth` and `eudkit synth` produce the same plan for the
      same Python fixture.
- [ ] `eudkit compile --plan` succeeds with no Python or Node.js on `PATH`.
- [ ] Plan and source builds produce byte-identical maps.
- [ ] `eudkit synth --check` detects drift.
- [ ] A malformed plan and a fingerprint mismatch produce diagnostics.
- [ ] `eudplib` no longer ships `eudkit` and declares the `eudkit` dependency.
- [ ] LSP advertises only domain features and passes the audit acceptance
      criteria.
- [ ] ADR-0006 records the TypeScript runtime outcome.

## 14. References

- ADR-0001: Two-tier IR in Rust.
- ADR-0003: TypeScript Frontend and migration CLI.
- ADR-0005: Trigger Loop timing model.
- ADR-0006: Host-Executed Frontends with Frozen HIR Plans.
- #54: Hover and signature help.
- #55: Lower reactive rules once across Python and TypeScript Frontends.
- #56: Frontend execution model decision and spike.
- `docs/architecture/lsp-audit.md`.
