# Scenario 15: Selected-Unit Field Reading — Architecture Comparison

## 1. Scenario Intent
Extracts UI telemetry for the local player's selected unit (`0x6284B8`):
- Converts the game pointer to an EPD handle.
- If selected, reads `hp`, `shield`, `unitType`, and `order`.
- Triggers a low-HP notification (`hp < 250`).
- Dispatches a state delta event when HP changes between consecutive frames (`hp != prev_hp`).
- Zeroes out all telemetry fields if no unit is selected.

---

## 2. Implementation Comparison

### Legacy v1 (`legacy.py`)
```python
sel_ptr = f_dwread(SELECTED_UNIT_PTR)
has_selection << 0

if EUDIf()(sel_ptr != 0):
    has_selection << 1
    cu = CUnit.from_ptr(sel_ptr)
    sel_hp << cu.hp
    sel_shield << cu.shield
    sel_unit_id << cu.unitType
    sel_order << cu.order
    ...
```

### Modern eudkit v2 (`v2.py`)
```python
# Direct pointer-to-EPD resolution + structured sub-field reads
sel_epd = epdread_epd(SELECTED_UNIT_EPD)

if sel_epd != EPD(0):
    has_selection = 1
    sel_hp = dwread_epd(sel_epd + 2)
    sel_shield = dwread_epd(sel_epd + 24)
    sel_unit_id = wread_epd(sel_epd + 25, 0)
    sel_order = bread_epd(sel_epd + 19, 3)
    ...
```

---

## 3. Quantitative Metrics

| Metric | Legacy v1 | JS Research Prototype | Modern eudkit v2 | Analysis |
| :--- | :--- | :--- | :--- | :--- |
| **Static Triggers** | 39 T (call sites only) | 1,075 T (unhinted materialization) | **116 T** (epdread + field offsets) | **-89.2% Triggers vs prototype** |
| **Global Hidden Engine** | 80 T ReadTable | 0 T | 0 T | Zero hidden dependencies |
| **True Total Map Triggers** | 119 T + injector | 1,075 T | **116 T** | **v2 wins in actual map footprint** |
| **Telemetry Resets** | Separate `if` block | Standard `else` branch | Standard `else` branch | Better locality |

---

## 4. Architectural Mechanics

### Overcoming Prototype Inflation via `epdread_epd`
- In the early research benchmark (`ours.js`), reading fields from a dynamic pointer generated a catastrophic **1,075 triggers** because the backend had to emit full unhinted 4-way-split address decoders for every single field read.
- Modern `eudkit` (v2) applies the **ideal intent pattern**: the game pointer at `0x6284B8` is converted to an EPD *once* via `epdread_epd`. All subsequent fields (`hp`, `shield`, `unitType`, `order`) are read using direct EPD arithmetic (`sel_epd + offset`). This drops the trigger footprint from 1,075 triggers down to **116 triggers** (an 89% reduction), achieving true parity with legacy runtime performance while maintaining standalone safety.

### Ergonomics & Locality
- Legacy `CUnit.from_ptr` creates an intermediate wrapper object that conceals how memory is dereferenced.
- Modern `eudkit` exposes explicit, typed sub-dword operations (`wread_epd`, `bread_epd`), ensuring transparent cost models and clean compilation boundaries.

### Architectural Mechanics Summary
- **Module Depth & Interface Design**: Legacy eudplib exposed a shallow interface leaking memory layouts across awkward seams, whereas modern eudkit provides deep modules that encapsulate trigger synthesis behind clean abstractions.
- **Locality & Compiler Leverage**: Local session variables ensure strict state locality, while eudkit compiler passes leverage AST analysis and reactive rules to minimize payload size.
