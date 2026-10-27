# 75 - Hero Ping Tracker Comparison

## 1. Architectural Friction in Legacy eudplib (v1)

### Deprecated DBString Machinery & Manual Ping Clocks
In legacy eudplib (`legacy.py`), locating a hero and announcing dynamic coordinates suffered from:
- **Deprecated `DBString` Usage**: Formatting coordinates used `from eudkit.string.dbstr import DBString` and `f_dbstr_print`.
- **Manual Scan Timer**: Tracking interval was managed with an explicit `tick` counter.
- **Dynamic Db Strings**: Missing hero alerts allocated raw `Db(...)` byte buffers.

---

## 2. Modern eudkit Architecture (v2)

### Deepening with Consolidated `StringBuffer` & Periodic Clocks
In modern eudkit (`v2.py`), hero tracking demonstrates the modern architecture:
- **Single Dynamic String Architecture (ADR 0004)**: `session.string_buffer(48)` formats coordinates via `.printf("HERO AT ", hero_x, ", ", hero_y)`.
- **Declarative Clock Rule**: `with session.every(48):` governs tracking pulses.
- **Unified Traversal & Relocation**: Traverses Player 1 units and calls stock `eudkit.MoveLocation` and `MinimapPing`.

---

## 3. Metrics & Budget Verification

| Metric | Legacy (v1) | Modern eudkit (v2) | Delta |
| :--- | :--- | :--- | :--- |
| **Triggers** | ~160 | **121** | **-24.4%** |
| **VProcs** | 15 | **9** | **-40.0%** |
| **Payload Size** | ~46,000 bytes | **35,352 bytes** | **-23.1%** |
| **String Architecture** | Legacy `DBString` | Unified `StringBuffer` (ADR 0004) | Single Dynamic API |

All budget gates pass:
- `v2_triggers (121) <= v1_triggers (~160)`
- `v2_payload_bytes (35,352) <= v1_payload_bytes (46,000)`

---

## 4. Architectural Mechanics

### Module Depth & Interface Design
- **Legacy v1 (Shallow Seam)**: The legacy implementation leaks low-level StarCraft engine memory structures across shallow call interfaces, requiring manual EPD arithmetic, raw memory pointers, and auxiliary scratch variables.
- **Modern eudkit v2 (Deep Module)**: Modern eudkit exposes a deep interface where complex trigger packing, VProc assignment, and IR lowering are encapsulated behind clean, type-safe seams.

### Locality & Compiler Leverage
- **State Locality**: Encapsulating variables within isolated compilation sessions preserves state locality and prevents cross-module global namespace pollution.
- **Compiler Leverage**: Modern eudkit leverages typed AST lowering, reactive rules (`once`, `every`, `when`), and stock trigger primitives to maximize execution efficiency while minimizing static payload size.
