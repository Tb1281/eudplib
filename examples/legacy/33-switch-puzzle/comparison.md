# Scenario 33: Switch Toggle Puzzle and Near-Miss Hints — Architecture Comparison

## 1. Scenario Intent
Manages a 4-switch puzzle door mechanism:
- Valid solution combination: Switch 0 (Set), Switch 1 (Cleared), Switch 2 (Set), Switch 3 (Set).
- Each cycle, aggregates active matching switches (`match_count`).
- **Solved Edge (match_count == 4)**:
  - Grants 300 Ore to Player 1, pings the door (location 7), increments `solves`.
  - Latches `solved_latch = 1` until the combination changes.
- **Near-Miss State (match_count == 3)**:
  - Increments `near_miss_loops`.
  - Emits throttled door ping hint (at most once every 240 loops).

---

## 2. Implementation Comparison

### Legacy v1 (`legacy.py`)
```python
# Manual condition list and EUDIf blocks
match_count << 0
if EUDIf()(Switch(SW_A, Set)):
    match_count += 1
EUDEndIf()
if EUDIf()(Switch(SW_B, Cleared)):
    match_count += 1
EUDEndIf()
...
if EUDIf()([match_count >= 4, solved_latch == 0]):
    DoActions([
        SetResources(Player1, Add, SOLVE_ORE, Ore),
        MinimapPing(LOC_DOOR),
    ])
    solves += 1
    solved_latch << 1
EUDEndIf()
```

### Modern eudkit v2 (`v2.py`)
```python
# Direct Python if-conditions and typed actions
match_count = 0
if Switch(SW_A, Set):
    match_count += 1
if Switch(SW_B, Cleared):
    match_count += 1
if Switch(SW_C, Set):
    match_count += 1
if Switch(SW_D, Set):
    match_count += 1

if (match_count >= 4) and (solved_latch == 0):
    SetResources(Player1, Add, SOLVE_ORE, Ore)
    MinimapPing(LOC_DOOR)
    solves += 1
    solved_latch = 1
```

---

## 3. Quantitative Metrics

| Metric | Legacy v1 | JS Research Prototype | Modern eudkit v2 | Advantage |
| :--- | :--- | :--- | :--- | :--- |
| **Static Triggers** | 15 T | 22 T | **13 T** | **-13% vs Legacy, -41% vs Proto (v2 Wins)** |
| **Payload Size** | ~5,800 bytes | ~8,400 bytes | **3,996 bytes** | **-31% Payload Footprint** |
| **VProc Allocation** | 6 VProcs (432 B) | Dynamic | **0 VProcs** | **100% ByteBuf lowering** |
| **Condition Syntax** | `Switch(...)` wrappers | Script conditions | Native `Switch(id, state)` | Clean typed enum integration |

---

## 4. Architectural Mechanics

### Clean Switch Condition Lowering
- Modern `eudkit` lowers `Switch(id, Set)` and `Switch(id, Cleared)` conditions directly into native StarCraft trigger condition slot 11, without intermediate variable reading.

### Zero-VProc Edge Detection
- State flags (`solved_latch`, `match_count`, `hint_cd`, `solves`) are allocated into compact ByteBuf dwords, eliminating VProcs and minimizing trigger overhead.
