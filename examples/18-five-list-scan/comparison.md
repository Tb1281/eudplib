# Scenario 18: Engine Unit List Census (Hidden Units) — Architecture Comparison

## 1. Scenario Intent
Surveys three distinct categories of units in StarCraft's memory architecture:
- (U) Live units actively linked in the main unit list (`0x628430`).
- (T) Hidden units: units present in the 1700-slot unit table that are valid (`unitType != 228`) but detached from the active live list (e.g. burrowed units, loaded cargo, or engine-cloaked entities).
- (S) Scanner Sweep units (`unitType == 33`).
- Tracks how many hidden units are Terran Marines.

---

## 2. Implementation Comparison

### Legacy v1 (`legacy.py`)
```python
# 1700-element EUDArray + EUDLoopUnit
mark = EUDArray(1700)
...
for ptr, epd in EUDLoopUnit():
    live_count += 1
    slot = (ptr - CUNIT_TABLE) // CUNIT_STRIDE
    mark[slot] << mark_gen

slot_idx = 0
EUDWhile()(slot_idx < 1700)
...
    if EUDIf()(mark[slot_idx] != mark_gen):
        hidden_count += 1
...
EUDEndWhile()
```

### Modern eudkit v2 (`v2.py`)
```python
# Compact memory Array + structured while loops
mark = Array(1700, epd=MARK_BUF_EPD, session=session)

cur_epd = epdread_epd(EPD(0x628430))
while cur_epd != EPD(0):
    live_count += 1
    slot = (cur_epd - CUNIT_BASE_EPD) // STRIDE
    mark[slot] = mark_gen
    cur_epd = epdread_epd(cur_epd + 1)

slot_idx = 0
while slot_idx < 1700:
    slot_epd = CUNIT_BASE_EPD + slot_idx * STRIDE
    unit_type = wread_epd(slot_epd + 25, 0)
    if unit_type != NONE_TYPE:
        if unit_type == SCANNER_SWEEP:
            scanner_count += 1
        if mark[slot_idx] != mark_gen:
            hidden_count += 1
            if unit_type == MARINE:
                hidden_marines += 1
    slot_idx += 1
```

---

## 3. Quantitative Metrics

| Metric | Legacy v1 | JS Research Prototype | Modern eudkit v2 | Analysis |
| :--- | :--- | :--- | :--- | :--- |
| **Static Triggers** | 120 T (call site delta) | 5,414 T (1700 VProcs!) | **183 T** (dense Array) | **-96.6% Triggers vs prototype** |
| **Array Trigger Cost** | 122,400 B (1700 VProcs) | 122,400 B (1700 VProcs) | **6,800 B (0 VProcs)** | **-94.4% Storage density** |
| **Global Hidden Engine** | 80 T ReadTable | 0 T | 0 T | Standalone IR |
| **Loop Control Flow** | Macro `EUDWhile()` | Prototype AST | Structured `while` | Clean AST |

---

## 4. Architectural Mechanics

### Eradicating the 5,400-Trigger Prototype Flaw
- In the early research benchmark (`ours.js`), this scenario generated a staggering **5,414 triggers** because the prototype allocated 1,700 separate `EUDVariable` instances for the mark table. Each variable expanded into a 72-byte VProc trigger with dedicated initialization actions, almost exhausting the map's entire trigger capacity in a single scenario.
- Modern `eudkit` backs `Array(1700)` with a compact 4-byte memory region. The mark array uses **0 VProcs**, bringing the trigger count down from 5,414 triggers to **183 triggers** (a 96.6% reduction).

### Generation-Based Slot Stamping
- Instead of clearing all 1,700 mark slots each loop (which would require 1,700 write operations per frame), both implementations increment `mark_gen` by 1 per frame. Any slot whose value doesn't match the current generation is recognized as unvisited in O(1) time.

### Architectural Mechanics Summary
- **Module Depth & Interface Design**: Legacy eudplib exposed a shallow interface leaking memory layouts across awkward seams, whereas modern eudkit provides deep modules that encapsulate trigger synthesis behind clean abstractions.
- **Locality & Compiler Leverage**: Local session variables ensure strict state locality, while eudkit compiler passes leverage AST analysis and reactive rules to minimize payload size.
