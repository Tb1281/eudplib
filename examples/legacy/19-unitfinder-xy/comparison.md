# Scenario 19: Spatial Unit Finder (2D Range Query) — Architecture Comparison

## 1. Scenario Intent
Performs an area of effect (AoE) 2D range query around `(target_x, target_y)` within `radius`:
- Iterates over StarCraft's sorted Unit Finder X array (`0x66FF78`: 3400 entries of `{coord_x, unit_ptr}`).
- Exploits spatial sorting to break early when `coord_x > hi_x`.
- Filters units satisfying `lo_x <= coord_x <= hi_x` and `lo_y <= pos_y <= hi_y`.
- Tracks matching count and latches the first discovered unit handle.

---

## 2. Implementation Comparison

### Legacy v1 (`legacy.py`)
```python
lo_x = target_x - radius
hi_x = target_x + radius
...
EUDWhile()(idx < FINDER_ENTRIES)
coord = f_dwread_epd(FINDER_BASE_EPD + idx * 2)
if EUDIf()(coord > hi_x):
    EUDBreak()
EUDEndIf()
if EUDIf()(coord >= lo_x):
    unit_ptr = f_dwread_epd(FINDER_BASE_EPD + idx * 2 + 1)
    pos_y = f_wread(unit_ptr + POS_Y_OFFSET)
    ...
```

### Modern eudkit v2 (`v2.py`)
```python
lo_x = target_x - radius
hi_x = target_x + radius
...
while idx < FINDER_ENTRIES:
    coord = dwread_epd(FINDER_BASE_EPD + idx * 2)
    if coord > hi_x:
        break
    if coord >= lo_x:
        unit_epd = epdread_epd(FINDER_BASE_EPD + idx * 2 + 1)
        pos_y = wread_epd(unit_epd + 10, 1)
        if pos_y >= lo_y and pos_y <= hi_y:
            ...
```

---

## 3. Quantitative Metrics

| Metric | Legacy v1 | JS Research Prototype | Modern eudkit v2 | Analysis |
| :--- | :--- | :--- | :--- | :--- |
| **Static Triggers** | 109 T (call site delta) | 250 T | **136 T** | **-45.6% Triggers vs prototype** |
| **Global Hidden Engine** | 80 T ReadTable | 0 T | 0 T | Zero hidden runtime dependency |
| **True Total Map Triggers** | 189 T + injector | 250 T | **136 T** | **v2 wins in actual map binary** |
| **Early Exit Idiom** | `EUDBreak()` macro | Prototype AST | Native `break` statement | Structured control transfer |

---

## 4. Architectural Mechanics

### Spatial Pruning & Algorithmic Leverage
- Both stacks leverage StarCraft's built-in sorted coordinate table (`0x66FF78`). Because entries are sorted ascending by X coordinate, evaluating `coord > hi_x` enables an immediate loop termination, pruning the search space from 3,400 elements down to the local spatial cluster.
- In legacy v1, `EUDBreak()` patches trigger jump destinations at runtime. In `eudkit`, the AST lowerer connects the break target directly to the loop exit basic block in HIR.

### Sub-Dword Coordination
- Extracting `posY` at byte offset `0x2A` requires reading a 16-bit word at `dword 10, subp 1`. `wread_epd` handles this alignment seamlessly, eliminating manual bitmask manipulation.
