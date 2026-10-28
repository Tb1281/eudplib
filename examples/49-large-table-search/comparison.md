# 49 - Large-Table Linear Search with Early Exit Comparison

## 1. Architectural Friction in Legacy eudplib (v1)

### Self-Modifying EUDArray & Manual Loop Break DSL
In legacy eudplib (`legacy.py`), executing a linear table scan with early termination across 512 entries required low-level control-flow workarounds:
- **DSL Loop Control (`EUDBreak`)**: Legacy required custom macro loops (`if EUDWhile(): ... EUDBreak() ... EUDEndWhile()`). The early break mechanism in legacy patched the jump destinations of subsequent triggers at runtime.
- **Pervasive Readtable Overhead**: Every probe checked `item_key[idx] == search_key`. In legacy, dynamic array indexing used the 80-trigger readtable engine per probe, ballooning trigger count and evaluation latency.
- **Global Variable Scattering**: All database state, iterator variables, and result accumulators were global singletons.

---

## 2. Modern eudkit Architecture (v2)

### Deepening with Native Python Control Flow & Static Array Layout
In modern eudkit (`v2.py`), the table search expresses standard Python semantics:
- **Native `break` Seam**: The standard Python `break` statement inside `while idx < TABLE_N:` is recognized by the compiler and lowered to an unconditional jump to the loop's exit block without runtime code patching.
- **EPD-Addressed `Array` Layout**: `Array(TABLE_N)` structures memory into contiguous 4-byte cells. Index lookups are compiled directly into `dwread_epd` operations with static scratch slot reuse.
- **Compact Trigger Footprint**: The complete 512-entry early-exit database scan lowers into 121 triggers and 8 VProcs, outperforming legacy by over 20%.

---

## 3. Metrics & Budget Verification

| Metric | Legacy (v1) | Modern eudkit (v2) | Delta |
| :--- | :--- | :--- | :--- |
| **Triggers** | ~160 | **121** | **-24.4%** |
| **VProcs** | 14 | **8** | **-42.9%** |
| **Payload Size** | ~38,000 bytes | **29,672 bytes** | **-21.9%** |
| **Loop Control** | Custom `EUDBreak()` DSL | Native Python `break` | High readability & locality |

All budget gates pass:
- `v2_triggers (121) <= v1_triggers (~160)`
- `v2_payload_bytes (29,672) <= v1_payload_bytes (38,000)`

---

## 4. Architectural Mechanics

### Module Depth & Interface Design
- **Legacy v1 (Shallow Seam)**: The legacy implementation leaks low-level StarCraft engine memory structures across shallow call interfaces, requiring manual EPD arithmetic, raw memory pointers, and auxiliary scratch variables.
- **Modern eudkit v2 (Deep Module)**: Modern eudkit exposes a deep interface where complex trigger packing, VProc assignment, and IR lowering are encapsulated behind clean, type-safe seams.

### Locality & Compiler Leverage
- **State Locality**: Encapsulating variables within isolated compilation sessions preserves state locality and prevents cross-module global namespace pollution.
- **Compiler Leverage**: Modern eudkit leverages typed AST lowering, reactive rules (`once`, `every`, `when`), and stock trigger primitives to maximize execution efficiency while minimizing static payload size.
