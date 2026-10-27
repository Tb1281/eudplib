# Scenario 08: Constant Lookup Table (Damage-by-Level) — Architecture Comparison

## 1. Scenario Intent
Performs lookup and threshold analysis from a 32-entry static damage progression table:
- Looks up the damage for the current player level (`damage = dmg_table[level]`).
- Tests if damage meets a fixed threshold (300) to illuminate a warning lamp.
- Looks up the damage for the next level (`dmg_table[level + 1]`).
- Performs a dynamic variable-versus-variable comparison (`next_dmg >= damage * 2`) to illuminate a critical lamp.

---

## 2. Implementation Comparison

### Legacy v1 (`legacy.py`)
```python
# Legacy EUDArray over Db-backed data
DMG_TABLE = EUDArray([10, 14, 19, ...])
...
damage << DMG_TABLE[level]

if EUDIf()(damage >= THRESHOLD):
    warn_lamp << 1
if EUDElse()():
    warn_lamp << 0
EUDEndIf()

next_dmg << DMG_TABLE[level + 1]
if EUDIf()(next_dmg >= damage * 2):
    crit_lamp << 1
if EUDElse()():
    crit_lamp << 0
EUDEndIf()
```

### Modern eudkit v2 (`v2.py`)
```python
# Modern compact memory Array with native Python indexing
dmg_table = Array(DMG_TABLE_DATA, epd=DMG_TABLE_EPD, session=session)
...
damage = dmg_table[level]

if damage >= THRESHOLD:
    warn_lamp = 1
else:
    warn_lamp = 0

next_dmg = dmg_table[level + 1]
if next_dmg >= damage * 2:
    crit_lamp = 1
else:
    crit_lamp = 0
```

---

## 3. Quantitative Metrics

| Metric | Legacy v1 | Modern eudkit v2 | Ratio | Analysis |
| :--- | :--- | :--- | :--- | :--- |
| **Early JS Prototype** | 21 T | **BLOCKED** (compiler bug) | — | Early prototype failed on var-vs-var comparison |
| **Modern eudkit (v2)** | 21 T (call sites only) | **112 T** (full standalone) | **5.33x** | See Engine Amortization note |
| **Table Storage Density** | 2,304 bytes (32 VProcs) | **128 bytes** (32 dwords) | **0.055x** | **18x denser storage (94.4% reduction)** |
| **Global Hidden Engine** | 80 T ReadTable | 0 T | — | Zero global runtime baggage |
| **True Total Map Triggers** | 101 T + injector | 112 T | **1.10x** | Complete parity |

---

## 4. Architectural Mechanics

### Breakthrough Over Early Prototype Flaws
- In the early JavaScript research benchmark (`ours.js`), this scenario was categorized as **BLOCKED**:
  1. The AST visitor failed to compile dynamic variable-versus-variable comparisons (`when(a.gte(b))` threw `PVar in patchable cond field 2 without an explicit filler declaration`).
  2. The table access emitter serialized stride constants as raw integer leaves rather than valid HIR addressing expressions.
- In modern `eudkit` (v2), the Rust backend and Python AST lowering pass solve both problems seamlessly. The expression `next_dmg >= damage * 2` correctly lowers to an inlined calculation of `damage * 2` followed by a dynamic condition patch using scratch VProc slots.

### Table Memory Footprint
- Legacy v1 uses `EUDArray`, which incurs a 72-byte trigger allocation per slot. A 32-entry lookup table consumes 2,304 bytes of trigger memory.
- Modern `eudkit` lowers static data tables to a contiguous 4-byte memory slice (128 bytes total), freeing critical map trigger capacity for gameplay logic.

### Architectural Mechanics Summary
- **Module Depth & Interface Design**: Legacy eudplib exposed a shallow interface leaking memory layouts across awkward seams, whereas modern eudkit provides deep modules that encapsulate trigger synthesis behind clean abstractions.
- **Locality & Compiler Leverage**: Local session variables ensure strict state locality, while eudkit compiler passes leverage AST analysis and reactive rules to minimize payload size.
