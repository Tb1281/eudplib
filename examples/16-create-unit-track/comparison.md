# Scenario 16: Unit Creation and Handle Tracking — Architecture Comparison

## 1. Scenario Intent
Periodically spawns a unit and tracks the created entity:
- Fires every 24 trigger frames (1 second).
- Inspects StarCraft's "First Empty Unit" pointer at `0x628438` before spawning.
- Calls `CreateUnit` to populate the slot.
- Modifies the newly spawned unit's HP to 512 and stores its handle for subsequent game systems.
- Emits milestone achievement flags when 10 units have been produced.

---

## 2. Implementation Comparison

### Legacy v1 (`legacy.py`)
```python
tick += 1
if EUDIf()(tick >= SPAWN_PERIOD):
    tick << 0
    next_ptr = f_dwread(NEXT_UNIT_PTR_ADDR)
    new_unit = CUnit.from_ptr(next_ptr)

    DoActions([CreateUnit(1, MARINE, SPAWN_LOCATION, Player1)])

    new_unit.hp << 512
    last_spawn_ptr << next_ptr
    spawn_count += 1
    last_spawn_hp << new_unit.hp
EUDEndIf()
```

### Modern eudkit v2 (`v2.py`)
```python
with every(24):
    next_epd = epdread_epd(NEXT_UNIT_EPD)
    CreateUnit(1, "Terran Marine", "Anywhere", Player1)
    dwwrite_epd(next_epd + 2, 512)
    last_spawn_epd = next_epd
    spawn_count += 1
    last_spawn_hp = dwread_epd(next_epd + 2)
```

---

## 3. Quantitative Metrics

| Metric | Legacy v1 | JS Research Prototype | Modern eudkit v2 | Analysis |
| :--- | :--- | :--- | :--- | :--- |
| **Static Triggers** | 22 T (call site delta) | 347 T | **106 T** | **-69.5% Triggers vs prototype** |
| **Timer Mechanism** | Manual `tick += 1` variable | Manual counter | Declarative `with every(24):` | Zero manual timer logic |
| **Global Hidden Engine** | 80 T ReadTable | 0 T | 0 T | Standalone execution |
| **VProc Allocation** | 5 VProcs (360 B) | Dynamic | 2 VProcs (144 B) | -60% VProcs |

---

## 4. Architectural Mechanics

### Reactive Timing Primitives
- **Legacy v1 (Manual Counter Burden)**: Legacy requires developers to allocate a dedicated variable (`tick`), increment it every frame, and branch on a comparison. This clutters the interface and introduces state pollution.
- **Modern v2 (Deep Reactive Rules)**: `with every(24):` delegates cyclic timer management to compiler-synthesized countdown triggers. The interface is completely declarative.

### Direct Handle Addressing
- Legacy wraps pointers into `CUnit.from_ptr` objects that carry hidden readtable dependencies.
- Modern `eudkit` reads the EPD directly via `epdread_epd` and performs direct field writes (`dwwrite_epd`), cutting prototype trigger bloat by nearly 70%.

### Architectural Mechanics Summary
- **Module Depth & Interface Design**: Legacy eudplib exposed a shallow interface leaking memory layouts across awkward seams, whereas modern eudkit provides deep modules that encapsulate trigger synthesis behind clean abstractions.
- **Locality & Compiler Leverage**: Local session variables ensure strict state locality, while eudkit compiler passes leverage AST analysis and reactive rules to minimize payload size.
