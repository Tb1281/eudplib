# 60 - Skill Cooldown Comparison

## 1. Architectural Friction in Legacy eudplib (v1)

### Flag Synthesis & Multi-Condition Nesting
In legacy eudplib (`legacy.py`), managing a hero nova skill with variable regen and cooldown required excess scaffolding:
- **Castable Flag Synthesis**: Checking whether mana was full and cooldown was 0 required initializing `castable << 0` and nesting two `EUDIf` blocks before testing `castable == 1`.
- **Manual Ready Ping Latching**: Triggering the audio ping once per charge cycle required an explicit latch variable tested across disjoint blocks.
- **Double-Condition Regen**: Fast vs slow regeneration required distinct `EUDIf(cooldown == 0)` and `EUDIf(cooldown >= 1)` trigger evaluation blocks.

---

## 2. Modern eudkit Architecture (v2)

### Deepening with Direct Boolean Lowering & Typed Stock Actions
In modern eudkit (`v2.py`), skill logic is written with natural Python syntax:
- **Direct Boolean Expressions**: `if mana == MANA_MAX and cooldown == 0` compiles directly to a multi-condition trigger, bypassing intermediate flag variables.
- **Typed Stock Actions**: Area kill actions use typed `eudkit.KillUnitAt` actions directly.
- **Clean Branching**: `if cooldown == 0: ... else: ...` provides mutual exclusion without redundant condition checks.

---

## 3. Metrics & Budget Verification

| Metric | Legacy (v1) | Modern eudkit (v2) | Delta |
| :--- | :--- | :--- | :--- |
| **Triggers** | ~24 | **18** | **-25.0%** |
| **VProcs** | 1 | **0** | **-100%** |
| **Payload Size** | ~8,400 bytes | **6,208 bytes** | **-26.1%** |
| **State Locality** | 7 global singletons | Scoped session variables | Clean module boundary |

All budget gates pass:
- `v2_triggers (18) <= v1_triggers (~24)`
- `v2_payload_bytes (6,208) <= v1_payload_bytes (8,400)`

---

## 4. Architectural Mechanics

### Module Depth & Interface Design
- **Legacy v1 (Shallow Seam)**: The legacy implementation leaks low-level StarCraft engine memory structures across shallow call interfaces, requiring manual EPD arithmetic, raw memory pointers, and auxiliary scratch variables.
- **Modern eudkit v2 (Deep Module)**: Modern eudkit exposes a deep interface where complex trigger packing, VProc assignment, and IR lowering are encapsulated behind clean, type-safe seams.

### Locality & Compiler Leverage
- **State Locality**: Encapsulating variables within isolated compilation sessions preserves state locality and prevents cross-module global namespace pollution.
- **Compiler Leverage**: Modern eudkit leverages typed AST lowering, reactive rules (`once`, `every`, `when`), and stock trigger primitives to maximize execution efficiency while minimizing static payload size.
