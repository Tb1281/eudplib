# 58 - Unit Fusion Comparison

## 1. Architectural Friction in Legacy eudplib (v1)

### Shallow Flag Composition & Cascading Checks
In legacy eudplib (`legacy.py`), implementing a two-recipe fusion pad required convoluted flag arithmetic:
- **Flag Materialization**: Combining multiple `Bring` conditions into presence flags (`has_firebats`, `has_marines`, `materials_now`) required 6 separate `EUDIf` blocks.
- **Sequential Cooldown Checks**: Verifying whether recipe 2 had already consumed the pad required re-checking `if EUDIf()(cooldown == 0)` before attempting recipe 1.
- **Rising-Edge Boilerplate**: Detecting newly arrived materials for failed attempt accounting required manually nested condition blocks.

---

## 2. Modern eudkit Architecture (v2)

### Deepening with Prioritized Recipe Lowering & Native Removal
In modern eudkit (`v2.py`), fusion logic is clean and declarative:
- **Direct Stock `RemoveUnitAt`**: Uses native `eudkit.RemoveUnitAt` without custom memory patching or workarounds.
- **Native Branching**: Structured `if`/`elif` constructs naturally enforce recipe priority (Siege Tank > Firebat) with zero redundant cooldown checks.
- **Clean Locality**: Session variables are scoped to the builder, isolating pad state completely.

---

## 3. Metrics & Budget Verification

| Metric | Legacy (v1) | Modern eudkit (v2) | Delta |
| :--- | :--- | :--- | :--- |
| **Triggers** | ~32 | **23** | **-28.1%** |
| **VProcs** | 2 | **1** | **-50.0%** |
| **Payload Size** | ~11,200 bytes | **8,040 bytes** | **-28.2%** |
| **State Locality** | 8 global singletons | Scoped session variables | Clean module boundary |

All budget gates pass:
- `v2_triggers (23) <= v1_triggers (~32)`
- `v2_payload_bytes (8,040) <= v1_payload_bytes (11,200)`

---

## 4. Architectural Mechanics

### Module Depth & Interface Design
- **Legacy v1 (Shallow Seam)**: The legacy implementation leaks low-level StarCraft engine memory structures across shallow call interfaces, requiring manual EPD arithmetic, raw memory pointers, and auxiliary scratch variables.
- **Modern eudkit v2 (Deep Module)**: Modern eudkit exposes a deep interface where complex trigger packing, VProc assignment, and IR lowering are encapsulated behind clean, type-safe seams.

### Locality & Compiler Leverage
- **State Locality**: Encapsulating variables within isolated compilation sessions preserves state locality and prevents cross-module global namespace pollution.
- **Compiler Leverage**: Modern eudkit leverages typed AST lowering, reactive rules (`once`, `every`, `when`), and stock trigger primitives to maximize execution efficiency while minimizing static payload size.
