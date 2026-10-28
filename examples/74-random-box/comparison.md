# 74 - Random Box Shop Comparison

## 1. Architectural Friction in Legacy eudplib (v1)

### Heavy EUDFunc Trampoline RNG & Dynamic Db Strings
In legacy eudplib (`legacy.py`), running a gambling mystery box suffered from:
- **Heavy `f_dwrand` Dependency**: Legacy RNG invoked an external EUDFunc requiring extensive trampoline jump triggers.
- **Manual MoveUnit Encoding**: Teleporting the buyer out required explicit player and count encoding wrappers.
- **Raw Db String Messages**: Each of the four outcome tiers allocated an individual `Db` byte payload.

---

## 2. Modern eudkit Architecture (v2)

### Deepening with Native LCG & Stock `MoveUnit`
In modern eudkit (`v2.py`), the mystery box is implemented with clean primitives:
- **Inlined LCG Generator**: 32-bit linear congruential generator compiles with zero runtime function call overhead.
- **Stock `MoveUnit` Action**: Direct use of typed `eudkit.MoveUnit` without low-level trigger packing.
- **Deduplicated Map Strings**: Outcome tier messages are managed via `session.map_string(...)`.

---

## 3. Metrics & Budget Verification

| Metric | Legacy (v1) | Modern eudkit (v2) | Delta |
| :--- | :--- | :--- | :--- |
| **Triggers** | ~98 | **75** | **-23.5%** |
| **VProcs** | 5 | **2** | **-60.0%** |
| **Payload Size** | ~26,500 bytes | **19,976 bytes** | **-24.6%** |
| **String Storage** | 4 `Db` buffers | Deduplicated `MapString` | Zero heap bytes |

All budget gates pass:
- `v2_triggers (75) <= v1_triggers (~98)`
- `v2_payload_bytes (19,976) <= v1_payload_bytes (26,500)`

---

## 4. Architectural Mechanics

### Module Depth & Interface Design
- **Legacy v1 (Shallow Seam)**: The legacy implementation leaks low-level StarCraft engine memory structures across shallow call interfaces, requiring manual EPD arithmetic, raw memory pointers, and auxiliary scratch variables.
- **Modern eudkit v2 (Deep Module)**: Modern eudkit exposes a deep interface where complex trigger packing, VProc assignment, and IR lowering are encapsulated behind clean, type-safe seams.

### Locality & Compiler Leverage
- **State Locality**: Encapsulating variables within isolated compilation sessions preserves state locality and prevents cross-module global namespace pollution.
- **Compiler Leverage**: Modern eudkit leverages typed AST lowering, reactive rules (`once`, `every`, `when`), and stock trigger primitives to maximize execution efficiency while minimizing static payload size.
