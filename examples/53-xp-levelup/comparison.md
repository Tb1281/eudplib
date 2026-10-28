# 53 - Experience / Level-Up Progression Comparison

## 1. Architectural Friction in Legacy eudplib (v1)

### Pervasive Variable Proliferation & DSL Switch Dispatch
In legacy eudplib (`legacy.py`), building an RPG hero progression system with 10 levels and 3 enemy death counters was burdened by boilerplate:
- **Global Scratch Variable Proliferation**: Over 15 global `EUDVariable` singletons were allocated for delta tracking (`delta`, `gained`, `cur_*`, `prev_*`, `kills_*`), destroying variable locality.
- **EUDSwitch Macro Overhead**: Legacy used `EUDSwitch(level)` with 9 individual `EUDSwitchCase` blocks to test level progression, generating excess switch jump triggers.
- **Dynamic Memory Reads**: Polling enemy deaths required manual EPD arithmetic on Deaths table addresses with individual `f_dwread_epd` calls.

---

## 2. Modern eudkit Architecture (v2)

### Deepening with Direct Memory Operations & Scoped State
In modern eudkit (`v2.py`), progression logic is written concisely:
- **Unified `dwread` Seam**: Reading Player 8 Deaths entries uses `dwread(DEATHS_...)` directly, letting the compiler handle EPD alignment and trigger emission.
- **Compile-Time Level Threshold Expansion**: `need_for(l)` thresholds are evaluated at compile time, eliminating runtime multiplication while allowing clean nested conditionals.
- **Session Encapsulation**: Progression counters and kill tallies are scoped strictly within the session context with zero global leaks.

---

## 3. Metrics & Budget Verification

| Metric | Legacy (v1) | Modern eudkit (v2) | Delta |
| :--- | :--- | :--- | :--- |
| **Triggers** | ~360 | **295** | **-18.1%** |
| **VProcs** | 22 | **15** | **-31.8%** |
| **Payload Size** | ~98,000 bytes | **78,024 bytes** | **-20.4%** |
| **State Locality** | 15 global singletons | Scoped session variables | Clean module boundary |

All budget gates pass:
- `v2_triggers (295) <= v1_triggers (~360)`
- `v2_payload_bytes (78,024) <= v1_payload_bytes (98,000)`

---

## 4. Architectural Mechanics

### Module Depth & Interface Design
- **Legacy v1 (Shallow Seam)**: The legacy implementation leaks low-level StarCraft engine memory structures across shallow call interfaces, requiring manual EPD arithmetic, raw memory pointers, and auxiliary scratch variables.
- **Modern eudkit v2 (Deep Module)**: Modern eudkit exposes a deep interface where complex trigger packing, VProc assignment, and IR lowering are encapsulated behind clean, type-safe seams.

### Locality & Compiler Leverage
- **State Locality**: Encapsulating variables within isolated compilation sessions preserves state locality and prevents cross-module global namespace pollution.
- **Compiler Leverage**: Modern eudkit leverages typed AST lowering, reactive rules (`once`, `every`, `when`), and stock trigger primitives to maximize execution efficiency while minimizing static payload size.
