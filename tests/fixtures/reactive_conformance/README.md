# Reactive conformance fixture

These files are the shared contract for the five reactive primitives (`once`,
`when`, `until`, `after`, `every`).

- `main.ts` uses the TypeScript callback forms.
- `main.py` uses the Python context-manager forms (`with once():`, `with when(...):`).
- `main_callbacks.py` uses the Python callback forms (`once(lambda: ...)`).
- `expected_rules.json` is the canonical sequence of `HirStmt::Reactive` rules
  emitted by all three programs.

Both frontends assert against `expected_rules.json`:

- `src/rust/eudkit-ts/tests/ts_compiler_test.rs` compiles `main.ts` through
  `compile_ts_to_hir`.
- `tests/test_eudkit_frontend.py` compiles `main.py` and `main_callbacks.py`
  through the public `Session` interface and reads the emitted HIR plan.

The fixture observes HIR, not trigger bytes. Guard and timer variables are
allocated later by the HIR reactive lowering module, so they are not part of
this contract.

When a reactive rule intentionally changes, regenerate `expected_rules.json`
from one frontend and verify the other still matches. Keep the variable
declaration order identical across the three programs so `VarId` values line
up in the golden file.
