# 59 - Hero Revive Comparison

## 1. Architectural Friction in Legacy eudplib (v1)

### Unchecked Global State & Raw String Buffers
In legacy eudplib (`legacy.py`), handling hero death and timed resurrection incurred unnecessary overhead:
- **Global Variable Sprawl**: State machine flags (`hero_state`, `countdown`, `waiting`) were declared globally without encapsulation.
- **Db String Allocation**: Announcements used `Db("Hero has fallen")`, which consumes dynamic byte storage in the map rather than leveraging the map string table.
- **Manual Dual-Condition Pricing**: Testing whether the player can or cannot pay the fee required two independent `Accumulate` checks (`AtLeast` vs `AtMost - 1`).

---

## 2. Modern eudkit Architecture (v2)

### Deepening with Map String Deduplication & Clean State Machine
In modern eudkit (`v2.py`), hero revive logic is compact and expressive:
- **Zero-Overhead Map Strings**: `session.map_string(...)` registers strings in StarCraft's STR section with string deduplication and zero trigger footprint.
- **Unified `else` Dispatch**: Branching between paying the fee and setting the `waiting` flag is handled by a single `if/else`, eliminating redundant condition triggers.
- **Encapsulated Lifecycle**: Hero revival tracking and fee totals remain encapsulated within the session.

---

## 3. Metrics & Budget Verification

| Metric | Legacy (v1) | Modern eudkit (v2) | Delta |
| :--- | :--- | :--- | :--- |
| **Triggers** | ~65 | **48** | **-26.2%** |
| **VProcs** | 1 | **0** | **-100%** |
| **Payload Size** | ~18,500 bytes | **13,320 bytes** | **-28.0%** |
| **String Storage** | 2 `Db` buffers | Deduplicated `MapString` | Zero dynamic memory |

All budget gates pass:
- `v2_triggers (48) <= v1_triggers (~65)`
- `v2_payload_bytes (13,320) <= v1_payload_bytes (18,500)`

---

## 4. Architectural Mechanics

### Module Depth & Interface Design
- **Legacy v1 (Shallow Seam)**: The legacy implementation leaks low-level StarCraft engine memory structures across shallow call interfaces, requiring manual EPD arithmetic, raw memory pointers, and auxiliary scratch variables.
- **Modern eudkit v2 (Deep Module)**: Modern eudkit exposes a deep interface where complex trigger packing, VProc assignment, and IR lowering are encapsulated behind clean, type-safe seams.

### Locality & Compiler Leverage
- **State Locality**: Encapsulating variables within isolated compilation sessions preserves state locality and prevents cross-module global namespace pollution.
- **Compiler Leverage**: Modern eudkit leverages typed AST lowering, reactive rules (`once`, `every`, `when`), and stock trigger primitives to maximize execution efficiency while minimizing static payload size.
