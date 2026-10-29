# Single Reactive Rule Ownership and Cross-Frontend Parity

`eudkit` defines each Reactive Rule (`once`, `when`, `until`, `after`, `every`) exactly once, in the HIR `ReactiveRule` enum and its canonical lowering pass. Frontends only produce `HirStmt::Reactive`; they do not own guard, timer, or firing semantics.

## Context

The five Reactive Rules were encoded on five surfaces: the Python context-manager builders, the Python `@eud.fn` AST lowering, the TypeScript SDK function bodies, the TypeScript compiler's reactive-call recognition, and the Rust HIR lowering pass. The TypeScript SDK bodies were a second semantic implementation. In particular, `when` evaluated `Boolean(condition)` on a build-time condition object, so executing that body would fire the callback unconditionally. The TypeScript test only counted `HirStmt::Reactive` nodes, so it could not detect the divergence.

ADR-0005 already fixes the timing model and the counter/guard lowering. This decision is about ownership: which layer is allowed to define that model.

## Decision

1. **One semantic owner.** `ReactiveRule` in `eudkit-core/src/hir.rs` and `eudkit-core/src/hir/reactive.rs` own the guard, timer, and firing behavior for all five rules.
2. **Frontends emit intent, not lowering.** The Python Frontend and the TypeScript Frontend produce `HirStmt::Reactive` and leave guard/timer allocation to the core pass.
3. **TypeScript SDK functions are compile-time markers.** `once`, `when`, `until`, `after`, and `every` contain no runtime implementation. Their bodies call a private `compileTimeOnlyReactive` helper that throws if a call escapes the compiler. The eudkit compiler recognizes the calls and emits Reactive HIR directly.
4. **Python keeps runtime builders.** Python executes the DSL at build time, so its context managers remain the Frontend adapter. Their overloaded argument shapes are normalized by one private helper, `_normalize_reactive_call`.
5. **Parity is tested in two phases.** A Python integration test drives equivalent Python and TypeScript fixtures through `eudkit dump-ir` and compares the pre-lowering `HirStmt::Reactive` blocks after stripping session-specific identifiers. The Rust `reactive_rules_test.rs` asserts the post-lowering guard and timer shapes.

## Consequences

- The Python and TypeScript Frontends cannot drift on guard or timer behavior; the shared fixture fails first if they do.
- A new Frontend has one integration obligation: emit `HirStmt::Reactive`. It does not re-implement timing.
- The TypeScript SDK is intentionally unusable outside the compiler. Calling a Reactive Rule from plain TypeScript now throws a clear error instead of silently producing a divergent trigger.
- The parity fixtures must be updated when the Reactive Rule surface changes, and the Rust shape test must be updated when the canonical lowering changes.
