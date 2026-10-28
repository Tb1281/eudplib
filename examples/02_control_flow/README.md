# Example 02: Control Flow

Comparison of conditionals and loops between **legacy `eudplib` (v1)** and **modern `eudkit` (v2)**.

## Architectural Comparison

| Concept | Legacy `eudplib` (v1) | Modern `eudkit` (v2) |
| :--- | :--- | :--- |
| **Control Syntax** | Leaky macro objects: `EUDIf()()`, `EUDEndIf()` | Idiomatic Python: `if`, `elif`, `else` |
| **Loop Syntax** | Leaky macros: `EUDWhile()()`, `EUDEndWhile()` | Idiomatic Python: `while`, `continue`, `break` |
| **Loop Unrolling** | Emitted trigger per iteration unconditionally | Compile-time loop unrolling + action coalescing |
| **Branch Inversion** | Manual jump triggers on every branch target | Automatic condition inversion to eliminate trampolines |
| **Trigger Count** | ~50-80 triggers for 10-iteration loop + branching | ~10-15 triggers due to action coalescing (up to 64/block) |

## Key Insights

### 1. Removing Macro Clutter
In v1, control flow leaked the internal mechanics of StarCraft triggers into Python source code (`if ep.EUDIf()(cond): ... ep.EUDEndIf()`), making IDE linters and syntax highlighters ineffective.
In v2, the Python frontend inspects the AST of `@session.fn` and lowers standard Python statements into high-leverage HIR basic blocks.

### 2. Action Coalescing
A single SCX trigger can store up to 64 actions. `eudkit` automatically merges consecutive actions belonging to the same basic block into single trigger blocks, slashing overall trigger payload by over **90%** compared to v1.

## Running the Examples

```bash
eudkit build --input tests/outputmap/basemap/basemap_strx.scx --output output_v2.scx --entry examples/02_control_flow/eudkit_v2.py
```
