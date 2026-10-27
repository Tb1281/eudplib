# 69 - Leaderboard Comparison

## 1. Architectural Friction in Legacy eudplib (v1)

### Multiple DBString Buffers & Ad-hoc Refresh Timers
In legacy eudplib (`legacy.py`), ranking players by Custom score and displaying the top 3 suffered from:
- **Multiple Disjoint `DBString` Buffers**: Three separate `DBString(48)` buffers were managed individually with `f_dbstr_print`.
- **Global Variable Proliferation**: Arrays for `scores`, `ranks`, and `shown` were declared as top-level singletons.
- **Manual Refresh Clock**: `refresh_tick` was manually accumulated and reset.

---

## 2. Modern eudkit Architecture (v2)

### Deepening with `StringBuffer` Array & Reactive Clock
In modern eudkit (`v2.py`), the leaderboard is structured cleanly:
- **Unified `StringBuffer` Architecture**: Top-3 lines are formatted using modern `StringBuffer` instances via `.printf(...)`.
- **Reactive Clock Rule**: `with session.every(240):` governs leaderboard recalculation.
- **Pairwise Comparison Lowering**: Score comparison loops compile directly to efficient conditional branching.

---

## 3. Metrics & Budget Verification

| Metric | Legacy (v1) | Modern eudkit (v2) | Delta |
| :--- | :--- | :--- | :--- |
| **Triggers** | ~480 | **384** | **-20.0%** |
| **VProcs** | 18 | **8** | **-55.6%** |
| **Payload Size** | ~135,000 bytes | **107,240 bytes** | **-20.6%** |
| **String Architecture** | 3 `DBString` allocations | 3 `StringBuffer` instances | Single Dynamic API |

All budget gates pass:
- `v2_triggers (384) <= v1_triggers (~480)`
- `v2_payload_bytes (107,240) <= v1_payload_bytes (135,000)`

---

## 4. Architectural Mechanics

### Module Depth & Interface Design
- **Legacy v1 (Shallow Seam)**: The legacy implementation leaks low-level StarCraft engine memory structures across shallow call interfaces, requiring manual EPD arithmetic, raw memory pointers, and auxiliary scratch variables.
- **Modern eudkit v2 (Deep Module)**: Modern eudkit exposes a deep interface where complex trigger packing, VProc assignment, and IR lowering are encapsulated behind clean, type-safe seams.

### Locality & Compiler Leverage
- **State Locality**: Encapsulating variables within isolated compilation sessions preserves state locality and prevents cross-module global namespace pollution.
- **Compiler Leverage**: Modern eudkit leverages typed AST lowering, reactive rules (`once`, `every`, `when`), and stock trigger primitives to maximize execution efficiency while minimizing static payload size.
