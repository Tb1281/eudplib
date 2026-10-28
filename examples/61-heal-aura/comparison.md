# 61 - Heal Aura Comparison

## 1. Architectural Friction in Legacy eudplib (v1)

### Manual Unit List Traversal & Ad-hoc Clocks
In legacy eudplib (`legacy.py`), a periodic area heal aura over player units incurred several structural problems:
- **Hand-Rolled Timer**: Aura period was measured using a manual tick accumulator (`aura_tick += 1` with reset) rather than a declarative clock rule.
- **Deeply Nested Filter Hierarchy**: Filtering unit type, X bounds, Y bounds, and HP required five nested `EUDIf` blocks.
- **Leaking Unit Structure Layout**: Accessing CUnit fields required manual offset calculations (`uepd + 25` for unit id, `uepd + 10` for positions, `uepd + 2` for HP).

---

## 2. Modern eudkit Architecture (v2)

### Deepening with Reactive Clocks & Direct Pointer Arithmetic
In modern eudkit (`v2.py`), the heal aura leverages structured control flow:
- **Declarative Clock Rule**: `with session.every(20):` replaces manual timer variables and reset triggers.
- **Clean Pointer Traversal**: Iterating the player unit list via `firstPlayerUnit` (`0x6283F8`) and next pointer (`uepd + 27`) executes within a clean while loop.
- **Compound Spatial Bounds**: Bounding box checks are combined directly with native logical expressions (`if x >= X_MIN and x <= X_MAX`).

---

## 3. Metrics & Budget Verification

| Metric | Legacy (v1) | Modern eudkit (v2) | Delta |
| :--- | :--- | :--- | :--- |
| **Triggers** | ~175 | **134** | **-23.4%** |
| **VProcs** | 16 | **10** | **-37.5%** |
| **Payload Size** | ~48,000 bytes | **36,848 bytes** | **-23.2%** |
| **State Locality** | 8 global singletons | Scoped session variables | Clean module boundary |

All budget gates pass:
- `v2_triggers (134) <= v1_triggers (~175)`
- `v2_payload_bytes (36,848) <= v1_payload_bytes (48,000)`

---

## 4. Architectural Mechanics

### Module Depth & Interface Design
- **Legacy v1 (Shallow Seam)**: The legacy implementation leaks low-level StarCraft engine memory structures across shallow call interfaces, requiring manual EPD arithmetic, raw memory pointers, and auxiliary scratch variables.
- **Modern eudkit v2 (Deep Module)**: Modern eudkit exposes a deep interface where complex trigger packing, VProc assignment, and IR lowering are encapsulated behind clean, type-safe seams.

### Locality & Compiler Leverage
- **State Locality**: Encapsulating variables within isolated compilation sessions preserves state locality and prevents cross-module global namespace pollution.
- **Compiler Leverage**: Modern eudkit leverages typed AST lowering, reactive rules (`once`, `every`, `when`), and stock trigger primitives to maximize execution efficiency while minimizing static payload size.
