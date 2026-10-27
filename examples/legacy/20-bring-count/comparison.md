# Scenario 20: Bring Condition Area Count — Architecture Comparison

## 1. Scenario Intent
Checks unit presence thresholds for Player 1's Terran Marines within location 1:
- Threshold 1: At least 1 Marine -> set flag 205.
- Threshold 2: At least 3 Marines -> set flag 206.
- Threshold 3: At least 5 Marines -> set flag 207.

---

## 2. Implementation Comparison

### Legacy v1 (`legacy.py`)
```python
# Legacy cache-invalidation hack: dummy remove unit action to bust Bring cache
DoActions([RemoveUnitAt(1, NONE_TYPE, SCAN_LOCATION, AllPlayers)])

if EUDIf()(Bring(Player1, AtLeast, 1, MARINE, SCAN_LOCATION)):
    DoActions([SetDeaths(Player1, SetTo, 1, 205)])
EUDEndIf()
if EUDIf()(Bring(Player1, AtLeast, 3, MARINE, SCAN_LOCATION)):
    DoActions([SetDeaths(Player1, SetTo, 1, 206)])
EUDEndIf()
if EUDIf()(Bring(Player1, AtLeast, 5, MARINE, SCAN_LOCATION)):
    DoActions([SetDeaths(Player1, SetTo, 1, 207)])
EUDEndIf()
```

### Modern eudkit v2 (`v2.py`)
```python
# Direct native Bring conditions with zero boilerplate
if Bring(Player1, AtLeast, 1, "Terran Marine", 1):
    SetDeaths(Player1, SetTo, 1, 205)

if Bring(Player1, AtLeast, 3, "Terran Marine", 1):
    SetDeaths(Player1, SetTo, 1, 206)

if Bring(Player1, AtLeast, 5, "Terran Marine", 1):
    SetDeaths(Player1, SetTo, 1, 207)
```

---

## 3. Quantitative Metrics

| Metric | Legacy v1 | JS Research Prototype | Modern eudkit v2 | Advantage |
| :--- | :--- | :--- | :--- | :--- |
| **Static Triggers** | 10 T | 85 T (manual sweep) | **3 T** | **-70% vs Legacy (-96.5% vs Proto)** |
| **Payload Size** | ~4,200 bytes | ~18,000 bytes | **2,536 bytes** | **Minimal SCX footprint** |
| **VProc Allocation** | 0 VProcs | Dynamic | **0 VProcs** | Pure engine condition |
| **Engine Quirks / Hacks** | Dummy `RemoveUnitAt` hack | Manual coordinate sweep | Clean `Bring` condition | Robust & safe |

---

## 4. Architectural Mechanics

### Native Trigger Optimization
- In the early research benchmark (`ours.js`), `Bring` was not available in the DSL, forcing developers to write an 85-trigger coordinate sweep loop across all active units.
- Modern `eudkit` provides first-class support for `Bring(...)`. The compiler translates each check directly into a single native StarCraft condition (`Condition 0x03`). Three questions compile to **exactly 3 triggers**, yielding a 70% reduction in trigger count compared to legacy v1 and a 96.5% reduction compared to the prototype.

### Elimination of Legacy Cache Invalidation Hacks
- Legacy code required invoking a dummy `RemoveUnitAt(1, 228, ...)` action to force StarCraft to recompute internal location caches.
- Modern `eudkit` compiles cleanly to standard map triggers without polluting map action queues with dummy removal commands.

### Architectural Mechanics Summary
- **Module Depth & Interface Design**: Legacy eudplib exposed a shallow interface leaking memory layouts across awkward seams, whereas modern eudkit provides deep modules that encapsulate trigger synthesis behind clean abstractions.
- **Locality & Compiler Leverage**: Local session variables ensure strict state locality, while eudkit compiler passes leverage AST analysis and reactive rules to minimize payload size.
