# Example 03: Functions & Inlining

Comparison of function declarations, inlining, and dynamic dispatch between **legacy `eudplib` (v1)** and **modern `eudkit` (v2)**.

## Architectural Comparison

| Concept | Legacy `eudplib` (v1) | Modern `eudkit` (v2) |
| :--- | :--- | :--- |
| **Decorator** | `@ep.EUDFunc`, `@ep.EUDTypedFunc` | `@session.fn(inline=...)` |
| **Return Mechanism** | `ep.EUDReturn(...)` macro | Standard Python `return ...` |
| **Small Helper Functions** | Incurs full 4+ trigger call overhead per invocation | Automatic compile-time inlining with 0 call overhead |
| **Calling Convention** | Caller copies args $\to$ callee, callee copies return $\to$ caller (double-copy) | **Direct-return convention**: callee writes directly to caller target slot (single action) |
| **Function Pointers** | `ep.EUDFuncPtr(args, rets)` | `session.func_ptr(args, rets, init=fn)` with `.set(new_fn)` |

## Key Insights

### 1. Inlining Small Functions
In StarCraft EUD maps, small utility calculations (e.g., coordinate conversions, unit distance calculations) frequently caused severe trigger bloat in v1.
In `eudkit`, marking a function with `inline=True` (or allowing the compiler to auto-inline leaf functions) substitutes the AST directly into callers with constant folding.

### 2. Direct-Return Calling Convention
For larger functions that cannot be inlined, v1 required a double-copy:
1. Caller writes parameters into global function registers.
2. Function executes.
3. Function writes return value into temporary return registers.
4. Caller copies return registers into destination variables.

`eudkit` eliminates the return register overhead: the caller passes a pointer to the target destination, and the function writes return values directly into that slot in a single action.

## Running the Examples

```bash
uv run python examples/03_functions_and_inlining/eudkit_v2.py
```
