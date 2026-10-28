# Scenario 07: Array Loop (Sum and In-Place Double) — Architecture Comparison

## 1. Scenario Intent
Iterates over a 16-element array each trigger frame:
- Pass 1: Accumulate all 16 values into `total`.
- Pass 2: Double every value in-place (`arr[idx] = arr[idx] * 2`).
- Finalize with a 32-bit checksum calculation (`total + (total >> 16)`).

---

## 2. Implementation Comparison

### Legacy v1 (`legacy.py`)
```python
# 72-byte VProc trigger array
arr = EUDArray([i + 1 for i in range(16)])
...
total << 0
idx << 0
if EUDWhile()(idx < 16):
    total << total + arr[idx]
    idx << idx + 1
EUDEndWhile()

idx << 0
if EUDWhile()(idx < 16):
    arr[idx] = arr[idx] * 2
    idx << idx + 1
EUDEndWhile()
```

### Modern eudkit v2 (`v2.py`)
```python
# Compact 4-byte cell Array with native Python loop
arr = Array([i + 1 for i in range(16)], epd=ARR_EPD, session=session)
...
total = 0
idx = 0
while idx < 16:
    total += arr[idx]
    idx += 1

idx = 0
while idx < 16:
    arr[idx] = arr[idx] * 2
    idx += 1

checksum = total + (total >> 16)
```

---

## 3. Quantitative Metrics

| Metric | Legacy v1 | Modern eudkit v2 | Ratio | Advantage |
| :--- | :--- | :--- | :--- | :--- |
| **Array Storage Footprint** | 1,152 bytes (16 VProcs) | **64 bytes** (16 dwords) | **0.055x** | **-94.4% Storage (18x denser)** |
| **Stride per Element** | 72 bytes / element | **4 bytes / element** | **0.055x** | Cache-friendly |
| **Static Triggers** | 27 T (call site delta) | 117 T (standalone) | **4.33x** | See Engine Amortization |
| **Global Hidden Engine** | 80 T ReadTable | 0 T | — | Self-contained IR |
| **True Total Map Triggers** | 107 T + injector | 117 T | **1.09x** | Comparable total triggers |

---

## 4. Architectural Mechanics

### Memory Model & Array Density
- **Legacy v1 (VProc Array Overhead)**: Legacy `EUDArray` maps each index slot to an entire 72-byte StarCraft trigger structure. For large arrays or tables, this rapidly exhausts the engine's hard trigger limit (maximum ~32,768 triggers per map).
- **Modern v2 (Dense Memory Buffer)**: `eudkit` backs `Array` with raw 4-byte dwords at byte offsets. A 16-element array consumes just 64 bytes instead of 1,152 bytes, achieving an **18x reduction** in memory density while offering standard subscript syntax `arr[idx]`.

### Control Flow Seam & Operator Ergonomics
- Legacy requires macro loop wrappers `EUDWhile()(cond)` and explicit assignment syntax `<<`.
- Modern v2 compiles native `while` loops, augmented assignments (`total += arr[idx]`), and binary operations (`total >> 16`) through clean AST visitor lowering.

### Architectural Mechanics Summary
- **Module Depth & Interface Design**: Legacy eudplib exposed a shallow interface leaking memory layouts across awkward seams, whereas modern eudkit provides deep modules that encapsulate trigger synthesis behind clean abstractions.
- **Locality & Compiler Leverage**: Local session variables ensure strict state locality, while eudkit compiler passes leverage AST analysis and reactive rules to minimize payload size.
