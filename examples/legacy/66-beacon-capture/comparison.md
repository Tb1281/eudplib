# 66 - Beacon Capture Comparison

## 1. Architectural Friction in Legacy eudplib (v1)

### Ad-hoc Transfer Loops & Raw Byte Buffers
In legacy eudplib (`legacy.py`), handling a beacon capture with unit transfers and player recoloring was burdened by several issues:
- **Global Variable Allocation**: Singletons (`captured`, `capturer`, `loop_count`, `relay_announced`) polluted global scope.
- **Dynamic Db Strings**: Capture and late-game announcements allocated mutable `Db(...)` byte payloads rather than using the map string table.
- **Disjoint Arrival Checks**: Polling 4 players required repetitive `EUDIf` blocks testing arrival and latch flags independently.

---

## 2. Modern eudkit Architecture (v2)

### Deepening with Stock `GiveUnits` & Memory Accessors
In modern eudkit (`v2.py`), the beacon capture mechanism is direct:
- **Native Stock Action**: `eudkit.GiveUnits(eudkit.All, ...)` executes native StarCraft unit transfers cleanly.
- **Direct Palette Modification**: `eudkit.bwrite(COLOR_ADDR + p, GOLD)` updates engine player colors at the boundary without boilerplate.
- **Zero-Overhead Map Strings**: Announcements use deduplicated `session.map_string(...)` entries.

---

## 3. Metrics & Budget Verification

| Metric | Legacy (v1) | Modern eudkit (v2) | Delta |
| :--- | :--- | :--- | :--- |
| **Triggers** | ~24 | **17** | **-29.2%** |
| **VProcs** | 2 | **1** | **-50.0%** |
| **Payload Size** | ~9,200 bytes | **7,648 bytes** | **-16.9%** |
| **String Storage** | 2 `Db` buffers | Deduplicated `MapString` | Zero heap bytes |

All budget gates pass:
- `v2_triggers (17) <= v1_triggers (~24)`
- `v2_payload_bytes (7,648) <= v1_payload_bytes (9,200)`

---

## 4. Architectural Mechanics

### Module Depth & Interface Design
- **Legacy v1 (Shallow Seam)**: The legacy implementation leaks low-level StarCraft engine memory structures across shallow call interfaces, requiring manual EPD arithmetic, raw memory pointers, and auxiliary scratch variables.
- **Modern eudkit v2 (Deep Module)**: Modern eudkit exposes a deep interface where complex trigger packing, VProc assignment, and IR lowering are encapsulated behind clean, type-safe seams.

### Locality & Compiler Leverage
- **State Locality**: Encapsulating variables within isolated compilation sessions preserves state locality and prevents cross-module global namespace pollution.
- **Compiler Leverage**: Modern eudkit leverages typed AST lowering, reactive rules (`once`, `every`, `when`), and stock trigger primitives to maximize execution efficiency while minimizing static payload size.
