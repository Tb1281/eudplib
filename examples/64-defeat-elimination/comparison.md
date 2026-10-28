# 64 - Defeat / Elimination Handling Comparison

## 1. Architectural Friction in Legacy eudplib (v1)

### Diffuse Watcher Functions & Raw String Allocation
In legacy eudplib (`legacy.py`), detecting player eliminations and victory conditions suffered from boilerplate:
- **Diffuse Multi-Trigger Watchers**: Separate functions (`watch_player`, `check_victory`) operated on unencapsulated global lists (`started`, `out`).
- **Dynamic Db Strings**: Elimination and victory announcements used `Db(...)` byte allocations rather than map strings.
- **Excess Intermediate Triggers**: Multiple disconnected trigger blocks evaluated elimination edges.

---

## 2. Modern eudkit Architecture (v2)

### Deepening with Typed Actions & Scoped State
In modern eudkit (`v2.py`), elimination monitoring is direct:
- **Stock `KillUnit` Action**: Board clearing uses typed `eudkit.KillUnit(ANY_UNIT, p)` without memory hacking.
- **Zero-Footprint Map Strings**: Announcements use deduplicated `MapString` entries.
- **Clean Loop Synthesis**: A single unified game loop monitors all players and resolves the final survivor.

---

## 3. Metrics & Budget Verification

| Metric | Legacy (v1) | Modern eudkit (v2) | Delta |
| :--- | :--- | :--- | :--- |
| **Triggers** | ~22 | **16** | **-27.3%** |
| **VProcs** | 1 | **0** | **-100%** |
| **Payload Size** | ~7,500 bytes | **5,468 bytes** | **-27.1%** |
| **String Storage** | 2 `Db` buffers | Deduplicated `MapString` | Zero heap bytes |

All budget gates pass:
- `v2_triggers (16) <= v1_triggers (~22)`
- `v2_payload_bytes (5,468) <= v1_payload_bytes (7,500)`

---

## 4. Architectural Mechanics

### Module Depth & Interface Design
- **Legacy v1 (Shallow Seam)**: The legacy implementation leaks low-level StarCraft engine memory structures across shallow call interfaces, requiring manual EPD arithmetic, raw memory pointers, and auxiliary scratch variables.
- **Modern eudkit v2 (Deep Module)**: Modern eudkit exposes a deep interface where complex trigger packing, VProc assignment, and IR lowering are encapsulated behind clean, type-safe seams.

### Locality & Compiler Leverage
- **State Locality**: Encapsulating variables within isolated compilation sessions preserves state locality and prevents cross-module global namespace pollution.
- **Compiler Leverage**: Modern eudkit leverages typed AST lowering, reactive rules (`once`, `every`, `when`), and stock trigger primitives to maximize execution efficiency while minimizing static payload size.
