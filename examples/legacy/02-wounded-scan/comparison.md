# Scenario 02: Wounded-Unit Scan — Architecture Comparison

## 1. Scenario Intent
Iterates over all active units in the game engine each frame and categorizes unit health:
- Reads the HP dword of each active unit (`CUnit + 0x08`, in fixed-point 1/256 HP units).
- Ignores dying/deleted units (`hp == 0`).
- Increments `wounded` count if HP < 50% (5120).
- Increments `critical` count if HP < 25% (2560).
- Latches the values into report variables and updates a binary `alarm` state.

---

## 2. Implementation Comparison

### Legacy v1 (`legacy.py`)
```python
# Legacy macro loop riding import-time readtable engine
for uptr, uepd in EUDLoopUnit():
    scanned += 1
    hp << f_dwread_epd(uepd + HP_EPD_OFFSET)
    if EUDIf()(hp == 0):
        EUDContinue()
    EUDEndIf()
    if EUDIf()(hp < WOUNDED_MAX):
        wounded += 1
        if EUDIf()(hp < CRITICAL_MAX):
            critical += 1
        EUDEndIf()
    EUDEndIf()

if EUDIf()(critical >= 1):
    alarm << 1
EUDEndIf()
if EUDIf()(critical == 0):
    alarm << 0
EUDEndIf()
```

### Modern eudkit v2 (`v2.py`)
```python
# Modern native while loop traversing linked-list pointers
cur_epd = epdread_epd(EPD(0x628430))
while cur_epd != EPD(0):
    cur_hp = dwread_epd(cur_epd + 2)
    if cur_hp > 0:
        scanned += 1
        if cur_hp < WOUNDED_MAX:
            wounded += 1
            if cur_hp < CRITICAL_MAX:
                critical += 1
    cur_epd = epdread_epd(cur_epd + 1)

report_scanned = scanned
report_wounded = wounded
report_critical = critical
if critical >= 1:
    alarm = 1
else:
    alarm = 0
```

---

## 3. Quantitative Metrics

| Metric | Legacy v1 | Modern eudkit v2 | Ratio | Analysis |
| :--- | :--- | :--- | :--- | :--- |
| **Static Triggers** | 51 T (call-site delta) | 114 T (standalone) | **2.23x** | See Engine Amortization note below |
| **Shared Hidden Triggers** | ~80 T (global readtable) | 0 T (self-contained) | — | Legacy hides ~80T in import-time engine |
| **True Total Map Triggers** | ~131 T | 114 T | **0.87x** | **v2 wins in total map footprint** |
| **VProc Allocation** | 8 VProcs (576 B) | 6 VProcs (scratch reused) | **0.75x** | -25% VProcs |
| **Control Flow Idiom** | `EUDIf() / EUDContinue()` | Native `while` + `if/else` | — | Eliminates control-flow macro bloat |

---

## 4. Architectural Mechanics

### Engine Amortization vs Self-Contained Locality
- **Legacy v1 (Hidden Global Seam)**: In legacy `eudplib`, the recorded count of 51 triggers is only the isolated delta of the call site. It critically depends on an 80-trigger global `ReadTable` and CP cache instantiated once at import time across the whole map. While this makes call sites appear lightweight, it leaks global state into every map build and prevents tree shaking or dead code elimination.
- **Modern v2 (Explicit Locality & Deep Module)**: `eudkit` lowers the pointer chase through explicit `epdread_epd` and `dwread_epd` mechanisms. The 114 triggers generated are completely self-contained, meaning zero hidden baggage is emitted when other modules don't need pointer dereferencing.

### Control Flow & Interface Expressiveness
- In legacy v1, premature loop skipping required `EUDContinue()`, which lowered into manual trigger jumping (`SetNextPtr`) that bypassed standard structured programming semantics.
- Modern v2 expresses the ideal intent via clean nesting (`if cur_hp > 0: ...`), which the AST compiler optimizes into consecutive conditional action sequences without requiring early branch escapes.
