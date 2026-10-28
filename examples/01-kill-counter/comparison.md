# Scenario 01: Kill Counter — Architecture Comparison

## 1. Scenario Intent
Implements an escalating-threshold kill reward system for Player 1:
- Each trigger cycle, the engine's Terran Marine death counter is checked against an escalating threshold (starting at 10 kills, increasing by +15 per payout).
- When the threshold is reached:
  - 100 Ore is awarded to Player 1.
  - The death counter is consumed (reset to 0).
  - Lifetime kills credited and payout statistics are recorded.
  - An additional wave milestone (+50 Ore) triggers every 3 payouts.

---

## 2. Implementation Comparison

### Legacy v1 (`legacy.py`)
```python
# Raw memory calculation leaking engine internals
MARINE_DEATHS_P1 = 0x58A364
...
if EUDIf()(rewards_paid == 0):
    on_first_frame()
EUDEndIf()

# Materializes memory into variable via shared readtable
cur_kills << f_dwread_epd(EPD(MARINE_DEATHS_P1))

if EUDIf()(cur_kills >= next_reward):
    # Direct memory write at raw ore offset 0x57F0F0
    DoActions(SetDeaths(EPD(0x57F0F0), Add, 100, 0))
    DoActions(SetDeaths(Player1, SetTo, 0, 0))
    ...
```

### Modern eudkit v2 (`v2.py`)
```python
# Isolated compilation session with typed variables
session = eudkit.Session(BASEMAP_PATH)
...
@session.fn
def game_loop():
    # Declarative one-time initialization
    with once():
        next_reward = 10
        rewards_this_wave = 0

    # In-engine condition evaluation against variable threshold
    if Deaths(Player1, AtLeast, next_reward, "Terran Marine"):
        SetResources(Player1, Add, 100, Ore)
        SetDeaths(Player1, SetTo, 0, "Terran Marine")
        kills_credited += next_reward
        next_reward += 15
        ...
```

---

## 3. Quantitative Metrics

| Metric | Legacy v1 | Modern eudkit v2 | Ratio | Advantage |
| :--- | :--- | :--- | :--- | :--- |
| **Static Triggers** | 25 T | 14 T | **0.56x** | **-44% Triggers** |
| **VProc Allocation** | 7 VProcs (504 B) | 3 VProcs (reused scratch) | **0.43x** | **-57% VProcs** |
| **Control Flow Overhead** | 3 `EUDIf` blocks | Native AST `if` + `with once():` | — | Zero boilerplate |
| **Memory Leaks** | Raw `0x57F0F0`, `0x58A364` | Fully Typed `Ore`, `Player1`, `Deaths` | — | Type safe |

---

## 4. Architectural Mechanics

### Module Depth & Interface Design
- **Legacy v1 (Shallow Seam)**: The legacy design forces developers to know low-level engine memory layouts (`0x58A364` for death tables, `0x57F0F0` for mineral balances, and manual `EPD()` arithmetic). The interface is shallow because it leaks internal engine structures to the caller, requiring boilerplate `DoActions` wrappers.
- **Modern v2 (Deep Module)**: `eudkit` provides a deep module interface (`Deaths`, `SetResources`, `SetDeaths`). The caller interacts solely with high-level game domain concepts (`Player1`, `"Terran Marine"`, `Ore`), while the compiler backend handles EPD resolution, action packing, and condition lowering behind a clean interface seam.

### Locality & In-Engine Evaluation
- In legacy v1, evaluating whether kills exceeded `next_reward` required two distinct steps:
  1. Materializing the death counter into a temporary `EUDVariable` via `f_dwread_epd` (2 triggers at call site, riding the 80-trigger import-time readtable).
  2. Performing a variable-versus-variable comparison (`cur_kills >= next_reward`) requiring conditional branching triggers.
- In modern v2, `Deaths(Player1, AtLeast, next_reward, "Terran Marine")` provides superior **locality**: it compiles directly into a native SC engine condition whose right-hand comparison value is dynamically filled by the compiler's VProc assignment. The value is evaluated directly within StarCraft's native trigger engine without materializing intermediate copies.

### Leverage of Reactive Contexts
- In legacy v1, one-time initialization required manual flag checking (`if EUDIf()(rewards_paid == 0): ... EUDEndIf()`), consuming triggers on every frame of the match.
- Modern v2 leverages `with once():`. The AST lowerer transforms this block into a self-disabling trigger gate that executes once and is permanently deactivated, providing maximal runtime leverage with minimal static payload.
