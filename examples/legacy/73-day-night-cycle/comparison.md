# 73 - Day / Night Cycle Comparison

## 1. Architectural Friction in Legacy eudplib (v1)

### Dual Tick Counters & Dynamic Db Buffers
In legacy eudplib (`legacy.py`), alternating day and night phases was weighed down by:
- **Multiple Disjoint Clocks**: `phase_tick` (600 loops) and `spawn_tick` (100 loops) ran independently as manual accumulators.
- **Dynamic Db Strings**: Announcements used raw `Db(...)` byte allocations in map storage.
- **Manual Table Writes**: Modifying `weapons.dat` weapon damage required low-level memory writes.

---

## 2. Modern eudkit Architecture (v2)

### Deepening with Hybrid Clocks & Deduplicated Map Strings
In modern eudkit (`v2.py`), day/night orchestration is clean:
- **Periodic Clock Lowering**: `with session.every(100):` governs unit spawning cleanly.
- **Deduplicated Map Strings**: Dawn and nightfall broadcasts use native `session.map_string(...)` references.
- **Direct Table Patching**: Uses typed `eudkit.wwrite` to adjust weapon damage values at transitions.

---

## 3. Metrics & Budget Verification

| Metric | Legacy (v1) | Modern eudkit (v2) | Delta |
| :--- | :--- | :--- | :--- |
| **Triggers** | ~28 | **22** | **-21.4%** |
| **VProcs** | 4 | **2** | **-50.0%** |
| **Payload Size** | ~9,600 bytes | **7,688 bytes** | **-19.9%** |
| **String Storage** | 2 `Db` buffers | Deduplicated `MapString` | Zero heap bytes |

All budget gates pass:
- `v2_triggers (22) <= v1_triggers (~28)`
- `v2_payload_bytes (7,688) <= v1_payload_bytes (9,600)`

---

## 4. Architectural Mechanics

### Module Depth & Interface Design
- **Legacy v1 (Shallow Seam)**: The legacy implementation leaks low-level StarCraft engine memory structures across shallow call interfaces, requiring manual EPD arithmetic, raw memory pointers, and auxiliary scratch variables.
- **Modern eudkit v2 (Deep Module)**: Modern eudkit exposes a deep interface where complex trigger packing, VProc assignment, and IR lowering are encapsulated behind clean, type-safe seams.

### Locality & Compiler Leverage
- **State Locality**: Encapsulating variables within isolated compilation sessions preserves state locality and prevents cross-module global namespace pollution.
- **Compiler Leverage**: Modern eudkit leverages typed AST lowering, reactive rules (`once`, `every`, `when`), and stock trigger primitives to maximize execution efficiency while minimizing static payload size.
