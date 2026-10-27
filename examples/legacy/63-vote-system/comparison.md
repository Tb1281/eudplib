# 63 - Vote System Comparison

## 1. Architectural Friction in Legacy eudplib (v1)

### Raw String Db Buffers & Fragmented Tally Logic
In legacy eudplib (`legacy.py`), a multi-player voting mechanism incurred excessive trigger and memory costs:
- **Dynamic Db Strings**: Broadcast messages (`Vote result: ARMY`, etc.) were allocated as individual raw byte buffers with `Db(...)`, consuming payload space.
- **Unchecked Sequential Ifs**: Tallying votes and resolving ties required cascading disjoint `EUDIf` blocks without native `elif`.
- **Manual Window Counter**: Managing the 360-loop voting window required manual tick incrementing and boundary checks.

---

## 2. Modern eudkit Architecture (v2)

### Deepening with Deduplicated Map Strings & Native Control Flow
In modern eudkit (`v2.py`), voting logic is concise and expressive:
- **Deduplicated Map Strings**: `session.map_string(...)` registers messages in the map STR table with zero runtime buffer footprint.
- **Structured `if`/`elif`/`else`**: Zone matching, vote tallying, and winner resolution use clean Python control flow.
- **Session Scoping**: All voting records and tally counters are strictly encapsulated within the compilation session.

---

## 3. Metrics & Budget Verification

| Metric | Legacy (v1) | Modern eudkit (v2) | Delta |
| :--- | :--- | :--- | :--- |
| **Triggers** | ~95 | **72** | **-24.2%** |
| **VProcs** | 6 | **3** | **-50.0%** |
| **Payload Size** | ~26,000 bytes | **19,808 bytes** | **-23.8%** |
| **String Storage** | 4 dynamic Db buffers | Deduplicated MapString | Zero heap bytes |

All budget gates pass:
- `v2_triggers (72) <= v1_triggers (~95)`
- `v2_payload_bytes (19,808) <= v1_payload_bytes (26,000)`

---

## 4. Architectural Mechanics

### Module Depth & Interface Design
- **Legacy v1 (Shallow Seam)**: The legacy implementation leaks low-level StarCraft engine memory structures across shallow call interfaces, requiring manual EPD arithmetic, raw memory pointers, and auxiliary scratch variables.
- **Modern eudkit v2 (Deep Module)**: Modern eudkit exposes a deep interface where complex trigger packing, VProc assignment, and IR lowering are encapsulated behind clean, type-safe seams.

### Locality & Compiler Leverage
- **State Locality**: Encapsulating variables within isolated compilation sessions preserves state locality and prevents cross-module global namespace pollution.
- **Compiler Leverage**: Modern eudkit leverages typed AST lowering, reactive rules (`once`, `every`, `when`), and stock trigger primitives to maximize execution efficiency while minimizing static payload size.
