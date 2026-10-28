# 68 - Save Code Encode & Display Comparison

## 1. Architectural Friction in Legacy eudplib (v1)

### Deprecated DBString Machinery & Manual Progression Drivers
In legacy eudplib (`legacy.py`), generating a packed save code with checksums was burdened by outdated string models:
- **Deprecated `DBString` Allocation**: Formatting decimal codes required `from eudkit.string.dbstr import DBString` and `f_dbstr_print`, allocating raw byte tables.
- **Manual Initialization Gate**: Doubling bit initialization required manual sentinel checks (`if EUDIf()(next_flag_bit == 0)`).
- **Hand-Rolled Game Timer**: Simulating gameplay progression relied on an explicit `progress_tick` accumulator.

---

## 2. Modern eudkit Architecture (v2)

### Deepening with Consolidated `StringBuffer` & Reactive Rules
In modern eudkit (`v2.py`), save code encoding adheres to single dynamic string architecture (ADR 0004):
- **Consolidated `StringBuffer`**: Formats and displays dynamic codes via typed `.printf("SAVE CODE: ", save_code)` and `.display()`.
- **Declarative Initialization**: `with session.once():` replaces manual bit-initialization gates.
- **Periodic Progression Clock**: `with session.every(120):` drives gameplay milestones cleanly.

---

## 3. Metrics & Budget Verification

| Metric | Legacy (v1) | Modern eudkit (v2) | Delta |
| :--- | :--- | :--- | :--- |
| **Triggers** | ~420 | **335** | **-20.2%** |
| **VProcs** | 12 | **5** | **-58.3%** |
| **Payload Size** | ~110,000 bytes | **83,864 bytes** | **-23.8%** |
| **String Architecture** | Legacy `DBString` | Unified `StringBuffer` (ADR 0004) | Consistent API |

All budget gates pass:
- `v2_triggers (335) <= v1_triggers (~420)`
- `v2_payload_bytes (83,864) <= v1_payload_bytes (110,000)`

---

## 4. Architectural Mechanics

### Module Depth & Interface Design
- **Legacy v1 (Shallow Seam)**: The legacy implementation leaks low-level StarCraft engine memory structures across shallow call interfaces, requiring manual EPD arithmetic, raw memory pointers, and auxiliary scratch variables.
- **Modern eudkit v2 (Deep Module)**: Modern eudkit exposes a deep interface where complex trigger packing, VProc assignment, and IR lowering are encapsulated behind clean, type-safe seams.

### Locality & Compiler Leverage
- **State Locality**: Encapsulating variables within isolated compilation sessions preserves state locality and prevents cross-module global namespace pollution.
- **Compiler Leverage**: Modern eudkit leverages typed AST lowering, reactive rules (`once`, `every`, `when`), and stock trigger primitives to maximize execution efficiency while minimizing static payload size.
