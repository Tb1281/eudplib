# Scenario 05: Byte and Word Read — Architecture Comparison

## 1. Scenario Intent
Performs sub-dword memory reads (16-bit words and 8-bit bytes) from both static memory locations and dynamic EPD pointers:
- Reads low byte (`subp=0`) and low word (`subp=0`) from score table address `0x58A364 + 0x40`.
- Reads `unitType` 16-bit word from `CUnit + 0x64`.
- Reads an 8-bit byte at `subp=2` through a dynamic EPD cursor.
- Combines the sub-fields into a single 32-bit status register using bitwise shifts.

---

## 2. Implementation Comparison

### Legacy v1 (`legacy.py`)
```python
# Legacy readers relying on global 80-trigger readtable engine
low_byte << f_bread_epd(FIXED_EPD, 0)
low_word << f_wread_epd(FIXED_EPD, 0)
unit_type << f_wread_epd(UNIT_TYPE_EPD, 0)
cursor_byte << f_bread_epd(cursor_epd, 2)

status << low_byte + (low_word << 8) + (unit_type << 16)
if EUDIf()(cursor_byte == 0xFF):
    status << status + 1
EUDEndIf()
```

### Modern eudkit v2 (`v2.py`)
```python
# Modern eudkit typed sub-dword reading through unified IR
low_byte = bread_epd(FIXED_EPD, 0)
low_word = wread_epd(FIXED_EPD, 0)
unit_type = wread_epd(UNIT_TYPE_EPD, 0)
cursor_byte = bread_epd(cursor_epd, 2)

status = low_byte + (low_word << 8) + (unit_type << 16)
if cursor_byte == 0xFF:
    status += 1
```

---

## 3. Quantitative Metrics

| Metric | Legacy v1 | Modern eudkit v2 | Ratio | Analysis |
| :--- | :--- | :--- | :--- | :--- |
| **Static Triggers** | 17 T (call sites only) | 105 T (self-contained) | **6.18x** | Isolated call sites vs full engine |
| **Global Hidden Engine** | 80 T (mandatory readtable) | 0 T (no global readtable overhead) | — | Legacy requires map-wide 80T |
| **True Total Triggers** | 97 T + injector | 105 T | **1.08x** | Near parity in real map binary |
| **VProc Allocation** | 6 VProcs (432 B) | 5 VProcs (reused scratch) | **0.83x** | -17% VProcs |
| **Syntax Overhead** | Macro `<<` and `EUDIf()` | Native `=` assignment and `if` | — | Zero boilerplate |

---

## 4. Architectural Mechanics

### Interface Seam & Amortization Trade-Off
- **Legacy v1 (Shared ReadTable Dependency)**: Legacy `f_bread_epd` and `f_wread_epd` achieve a 17-trigger call-site footprint by delegating the actual bit extraction to an 80-trigger `ReadTable` pre-injected into the map during `import eudkit`. If a map only performs one byte read, it still bears the entire 80-trigger burden.
- **Modern v2 (Modular Code Generation)**: `eudkit` models sub-dword extraction as `MemRead` operations in HIR/LIR. The compiler emits dedicated bitmask operations and VProc assignments tailored to the requested bit width (8-bit or 16-bit) and sub-dword alignment (subp 0..3), without coupling the scenario to a global runtime singleton.

### Expression Locality & Assignment Ergonomics
- Legacy variable mutation required `<<` operator semantics, and conditional increments required `EUDIf() ... EUDEndIf()` wrappers.
- Modern `eudkit` lowers native assignment (`status = ...`) and standard Python augmented assignment (`status += 1`) directly through AST parsing, optimizing memory writes to single-cycle trigger actions.

### Architectural Mechanics Summary
- **Module Depth & Interface Design**: Legacy eudplib exposed a shallow interface leaking memory layouts across awkward seams, whereas modern eudkit provides deep modules that encapsulate trigger synthesis behind clean abstractions.
- **Locality & Compiler Leverage**: Local session variables ensure strict state locality, while eudkit compiler passes leverage AST analysis and reactive rules to minimize payload size.
