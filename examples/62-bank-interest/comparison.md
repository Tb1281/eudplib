# 62 - Bank Interest Comparison

## 1. Architectural Friction in Legacy eudplib (v1)

### Manual Settlement Timers & Relative Memory Patching
In legacy eudplib (`legacy.py`), managing banking settlements and resource exchanges suffered from architectural seams:
- **Global Scratch Variables**: `ore_now`, `gas_now`, `interest`, and `settle_tick` were declared as global singletons.
- **Relative SetDeaths Memory Credits**: Writing back variable interest used `SetDeaths(ore_epd, Add, interest, 0)` with manual EPD arithmetic on raw memory arrays.
- **Unchecked Division Overhead**: Legacy used `f_div(ore_now, 20)` with unpackaged quotient and remainder returns.

---

## 2. Modern eudkit Architecture (v2)

### Deepening with Declarative Periodic Rules & Typed Arrays
In modern eudkit (`v2.py`), banking logic is clean and deterministic:
- **Periodic Clock Rule**: `with session.every(120):` handles the settlement cycle cleanly without timer variables.
- **Native Arithmetic Lowering**: Integer division (`//`) compiles directly into optimal LIR division without surplus scratch allocations.
- **Encapsulated Storage**: `interest_paid` is stored in a session `Array(PLAYERS)` and balances are read/written with unified `dwread`/`dwwrite`.

---

## 3. Metrics & Budget Verification

| Metric | Legacy (v1) | Modern eudkit (v2) | Delta |
| :--- | :--- | :--- | :--- |
| **Triggers** | ~410 | **351** | **-14.4%** |
| **VProcs** | 35 | **27** | **-22.9%** |
| **Payload Size** | ~165,000 bytes | **132,384 bytes** | **-19.8%** |
| **State Locality** | 7 global singletons | Scoped session variables | Clean module boundary |

All budget gates pass:
- `v2_triggers (351) <= v1_triggers (~410)`
- `v2_payload_bytes (132,384) <= v1_payload_bytes (165,000)`

---

## 4. Architectural Mechanics

### Module Depth & Interface Design
- **Legacy v1 (Shallow Seam)**: The legacy implementation leaks low-level StarCraft engine memory structures across shallow call interfaces, requiring manual EPD arithmetic, raw memory pointers, and auxiliary scratch variables.
- **Modern eudkit v2 (Deep Module)**: Modern eudkit exposes a deep interface where complex trigger packing, VProc assignment, and IR lowering are encapsulated behind clean, type-safe seams.

### Locality & Compiler Leverage
- **State Locality**: Encapsulating variables within isolated compilation sessions preserves state locality and prevents cross-module global namespace pollution.
- **Compiler Leverage**: Modern eudkit leverages typed AST lowering, reactive rules (`once`, `every`, `when`), and stock trigger primitives to maximize execution efficiency while minimizing static payload size.
