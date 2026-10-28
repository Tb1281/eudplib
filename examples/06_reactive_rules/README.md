# Example 06: Reactive Rules & Event Scheduling

Comparison of reactive event scheduling and timer logic between **legacy `eudplib` (v1)** and **modern `eudkit` (v2)**.

## Architectural Comparison

| Concept | Legacy `eudplib` (v1) | Modern `eudkit` (v2) |
| :--- | :--- | :--- |
| **Event Paradigm** | Manual tick polling and state flags | Declarative reactive blocks (`with once():`, `with every(N):`) |
| **Language Parity** | Only imperative python syntax | Python context managers (`with once():`) & JS callbacks (`once(() => {...})`) |
| **Timer State** | Mapmakers declare & decrement `EUDVariable` per timer | Compiler allocates and manages clock triggers internally |
| **Delay Safety** | Fragile `DoActions(Wait(1000))` (causes MP desync) | Multiplayer-safe frame-tick counters (`with after(N):`) |
| **Condition Bounds** | Nested flag branches (`if init_once == 1:`) | Native reactive conditions (`with when(cond):`, `with until(cond):`) |
| **Trigger Count** | 4–8 triggers per timer state machine | Minimal compiled trigger footprint with action coalescing |

## Key Differences

### 1. Declarative Reactive Blocks vs Manual State Machines
In v1, running code once, periodically, or after a delay required manual state machines:
```python
# Legacy v1: Manual variable bookkeeping for a 1-second periodic timer
timer_24 = ep.EUDVariable(24)
if ep.EUDIf()(timer_24 > 0):
    timer_24 -= 1
ep.EUDEndIf()
if ep.EUDIf()(timer_24 == 0):
    timer_24 << 24
    ep.DoActions(ep.SetResources(ep.Player1, ep.Add, 50, ep.Ore))
ep.EUDEndIf()
```

In v2, the same logic is expressed cleanly using idiomatic language constructs:
```python
# Modern v2 (Python): Context manager syntax
with once():
    DisplayText("Game Initialized once")

with every(24):
    SetResources(Player1, Add, 50, Ore)
```

```typescript
// Modern v2 (TypeScript): Callback syntax
once(() => {
    DisplayText("Game Initialized once");
});

every(24, () => {
    SetResources(Player1, Add, 50, Ore);
});
```

### 2. Elimination of Multiplayer Desyncs from `Wait()`
In StarCraft: Brood War, the native `Wait()` action freezes trigger thread execution across game frames. In multiplayer games or maps with multiple trigger owners, `Wait()` frequently causes desynchronization and corrupted trigger queues.

`eudkit` v2 replaces manual delays with deterministic frame-tick reactive rules (`with after(loops):` / `after(loops, () => ...)`), compiling them into safe, non-blocking condition/action sets that never stall trigger execution.

### 3. Procedural Branching (`if`) vs Declarative Rules (`when`)
Inside game loop routines (`@session.fn` in Python, or `export function main()` in TypeScript), native `if / else if / else` is the preferred, idiomatic way to write conditional branch logic. Both frontends compile native `if` statements directly into StarCraft trigger conditions via AST lowering.

Declarative rules (`with when(cond):` / `when(cond, () => ...)`) are retained as part of the complete 5-primitive reactive suite (`once`, `every`, `after`, `until`, `when`). They provide clean semantic symmetry with `until` and allow declaring standing polling constraints without managing procedural loop state.

## Running the Examples

```bash
# Build using eudkit build:
eudkit build --input tests/outputmap/basemap/basemap_strx.scx --output output_v2.scx --entry examples/06_reactive_rules/eudkit_v2.py
```
