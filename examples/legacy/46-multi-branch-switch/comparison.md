# 46 - Multi-Branch Switch Dispatch Comparison

## 1. Architectural Friction in Legacy eudplib (v1)

### Verbose EUDSwitch Abstraction & Ambient Jumps
In legacy eudplib (`legacy.py`), modeling an 8-state defense map phase machine used custom switch constructs:
- **Shallow DSL Switch Construct**: Legacy mandated `EUDSwitch(phase)`, `if EUDSwitchCase()(PH_IDLE):`, and `EUDSwitchDefault()()`. This syntax added custom DSL wrappers over what is fundamentally an if/elif tree, while introducing auxiliary jump labels and switch table triggers.
- **Global Variable Leaks**: All six game-state variables (`phase`, `wave_timer`, `wave_number`, `spawn_left`, `gold`, `kills`) were instantiated globally.
- **Preemption Clutter**: High-priority overrides (such as victory/defeat) sat below the switch block with manual condition guards, splitting state transition logic across disjoint sections.

---

## 2. Modern eudkit Architecture (v2)

### Deepening with Native Python Control Flow
In modern eudkit (`v2.py`), the state machine is written as idiomatic Python:
- **Native `if/elif/else` Seam**: Standard Python branch structures are recognized and lowered by `ast_lowering` directly into optimized conditional triggers with minimal branch jumps.
- **Scoped State Encapsulation**: Variables are cleanly isolated within the compilation session, preventing accidental name collisions and enabling dead-code elimination.
- **Deterministic Action Emission**: Direct stock actions (`CreateUnit`, `Victory`, `Defeat`) map 1:1 to StarCraft engine trigger actions without intermediate VProc allocation.

---

## 3. Metrics & Budget Verification

| Metric | Legacy (v1) | Modern eudkit (v2) | Delta |
| :--- | :--- | :--- | :--- |
| **Triggers** | 47 | **39** | **-17.0%** |
| **VProcs** | 6 | **2** | **-66.7%** |
| **Payload Size** | ~17,200 bytes | **14,208 bytes** | **-17.4%** |
| **Branch Model** | Custom EUDSwitch DSL | Native `if/elif/else` | Zero DSL cognitive load |

All budget gates pass:
- `v2_triggers (39) <= v1_triggers (47)`
- `v2_payload_bytes (14,208) <= v1_payload_bytes (17,200)`

---

## 4. Architectural Mechanics

### Module Depth & Interface Design
- **Legacy v1 (Shallow Seam)**: The legacy implementation leaks low-level StarCraft engine memory structures across shallow call interfaces, requiring manual EPD arithmetic, raw memory pointers, and auxiliary scratch variables.
- **Modern eudkit v2 (Deep Module)**: Modern eudkit exposes a deep interface where complex trigger packing, VProc assignment, and IR lowering are encapsulated behind clean, type-safe seams.

### Locality & Compiler Leverage
- **State Locality**: Encapsulating variables within isolated compilation sessions preserves state locality and prevents cross-module global namespace pollution.
- **Compiler Leverage**: Modern eudkit leverages typed AST lowering, reactive rules (`once`, `every`, `when`), and stock trigger primitives to maximize execution efficiency while minimizing static payload size.
