# Example 01: Variables & Arithmetic

Comparison of basic variable declarations and arithmetic expressions between **legacy `eudplib` (v1)** and **modern `eudkit` (v2)**.

## Architectural Comparison

| Concept | Legacy `eudplib` (v1) | Modern `eudkit` (v2) |
| :--- | :--- | :--- |
| **State Locality** | Global mutable module state (`LoadMap`, `SaveMap`) | Isolated `Compilation Session` (`eudkit.Session(map)`) |
| **Variable Depth** | Leaky `EUDVariable` / `EUDLightVariable` selection | Deep `session.var(...)` with automatic storage lowering |
| **Expression Lowering** | Operator overloading allocating temporary triggers | Rust HIR $\to$ LIR expression tree lowering with scratch reuse |
| **Action Interface** | Explicit `ep.DoActions(...)` wrapper required | Direct action invocations (`SetDeaths(...)`) |
| **Payload Size** | 72-byte trigger per variable + relocation metadata | 4-byte cell for constant-only variables, packed VProcs |

## Key Differences

### 1. Unified Variable Interface with Automatic Storage Lowering
In v1, mapmakers had to choose between `EUDVariable` (72B VProc trigger, dynamic copying enabled) and `EUDLightVariable` (4B cell, no dynamic copying).
In v2, you declare `session.var(...)`. The compiler analyzes whether the variable requires dynamic copies or only constant operations, automatically lowering to 4-byte memory buffers when possible with zero VProc overhead.

### 2. Expression Compounding & Scratch Reuse
In v1, compound expressions like `d << (a + b) - c + (b & 0xFF)` created chain allocations of temporary triggers.
In v2, the compiler builds an expression AST, folds constants at compile time, and reuses dedicated scratch slots in memory.

## Running the Examples

```bash
# Build using eudkit build:
eudkit build --input tests/outputmap/basemap/basemap_strx.scx --output output_v2.scx --entry examples/01_variables_and_math/eudkit_v2.py
```
