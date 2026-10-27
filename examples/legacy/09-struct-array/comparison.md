# Scenario 09: Struct-Array Element Field Access — Architecture Comparison

## 1. Scenario Intent
Manages an 8-slot table of structured unit records (each 84 dwords long, mirroring CUnit memory layout):
- Pass 1 (Respawn allocation): Searches for the first empty slot (`hp == 0`), initializes HP to 1000, stores the 1-based index in `respawned`, and breaks early.
- Pass 2 (Energy decay): Iterates over all active records (`hp != 0`), decrementing `energy` by 1 if `energy >= 1`.

---

## 2. Implementation Comparison

### Legacy v1 (`legacy.py`)
```python
records = Db(b"\x00" * (8 * STRIDE * 4))
rec_epd = EPD(records)
...
# Pass 1 with manual EUDBreak
idx << 0
if EUDWhile()(idx < 8):
    slot_epd = rec_epd + idx * STRIDE
    hp = f_dwread_epd(slot_epd + DW_HP)
    if EUDIf()(hp == 0):
        f_dwwrite_epd(slot_epd + DW_HP, 1000)
        respawned << idx + 1
        EUDBreak()
    EUDEndIf()
    idx << idx + 1
EUDEndWhile()
```

### Modern eudkit v2 (`v2.py`)
```python
# Pass 1 with native Python break in structured while loop
idx = 0
respawned = 0
while idx < 8:
    slot_epd = REC_BUF_EPD + idx * STRIDE
    hp = dwread_epd(slot_epd + DW_HP)
    if hp == 0:
        dwwrite_epd(slot_epd + DW_HP, 1000)
        respawned = idx + 1
        break
    idx += 1
```

---

## 3. Quantitative Metrics

| Metric | Legacy v1 | Modern eudkit v2 | Ratio | Analysis |
| :--- | :--- | :--- | :--- | :--- |
| **Static Triggers** | 64 T (call sites only) | 187 T (standalone) | **2.92x** | See Engine Amortization note |
| **JS Prototype Baseline** | 256 T | 187 T | **0.73x** | **-27% Triggers vs early prototype** |
| **Global Hidden Engine** | 80 T ReadTable | 0 T | — | Zero hidden dependencies |
| **Loop Break Idiom** | `EUDBreak()` macro | Native `break` AST statement | — | Structured control flow |

---

## 4. Architectural Mechanics

### Structured Control Flow & Early Loop Termination
- **Legacy v1 (Jump Hack)**: In legacy `eudplib`, breaking out of a loop requires `EUDBreak()`. Under the hood, this relies on rewriting trigger jump targets (`SetNextPtr`) dynamically at runtime, creating brittle non-local control transfer seams.
- **Modern v2 (AST Loop Control Flow)**: `eudkit` lowers Python `break` statements natively into the HIR block model. The compiler guarantees that all cleanup actions and block terminators are placed in the correct target basic block without pointer mutation.

### Storage and Memory Addressing
- Both stacks perform identical offset calculations (`base_epd + idx * STRIDE + field_offset`). However, modern `eudkit` eliminates redundant VProc allocations by scratch-reusing dynamic calculation variables (`slot_epd`, `hp`, `energy`), maintaining high compiler leverage.
