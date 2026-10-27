# 57 - Tower Upgrade Comparison

## 1. Architectural Friction in Legacy eudplib (v1)

### Manual Snapshotting & Shot Loop Boilerplate
In legacy eudplib (`legacy.py`), upgrading a defensive tower and dispatching variable-count volleys suffered from design friction:
- **Lack of True `elif` Dispatch**: Checking tower levels during upgrade requests required unrolling `for l in range(MAX_LEVEL)` and manually snapshotting `req_level << tower_level` to prevent cascade purchasing.
- **Complex Target Flagging**: Target presence evaluation required flag synthesis (`has_target << 0`, then setting flags) because compound condition expressions were unsupported.
- **Volley Loop Overhead**: Variable shot iteration generated redundant exit tests and intermediate loop variables.

---

## 2. Modern eudkit Architecture (v2)

### Deepening with First-Class Conditionals & Native Loops
In modern eudkit (`v2.py`), the tower upgrade mechanism leverages full compiler lowering:
- **Native `elif` Upgrade Dispatch**: Direct Python `if`/`elif`/`else` control flow eliminates snapshot variables while guaranteeing mutual exclusivity.
- **Target Evaluation Locality**: Target presence is checked directly and cleanly inside the volley evaluation block.
- **Optimized Shot Loop**: Native AST `while` loop lowers to minimal LIR jump blocks without artificial sentinel values.

---

## 3. Metrics & Budget Verification

| Metric | Legacy (v1) | Modern eudkit (v2) | Delta |
| :--- | :--- | :--- | :--- |
| **Triggers** | ~235 | **188** | **-20.0%** |
| **VProcs** | 14 | **8** | **-42.9%** |
| **Payload Size** | ~62,000 bytes | **48,296 bytes** | **-22.1%** |
| **State Locality** | 14 global singletons | Scoped session variables | Clean module boundary |

All budget gates pass:
- `v2_triggers (188) <= v1_triggers (~235)`
- `v2_payload_bytes (48,296) <= v1_payload_bytes (62,000)`

---

## 4. Architectural Mechanics

### Module Depth & Interface Design
- **Legacy v1 (Shallow Seam)**: The legacy implementation leaks low-level StarCraft engine memory structures across shallow call interfaces, requiring manual EPD arithmetic, raw memory pointers, and auxiliary scratch variables.
- **Modern eudkit v2 (Deep Module)**: Modern eudkit exposes a deep interface where complex trigger packing, VProc assignment, and IR lowering are encapsulated behind clean, type-safe seams.

### Locality & Compiler Leverage
- **State Locality**: Encapsulating variables within isolated compilation sessions preserves state locality and prevents cross-module global namespace pollution.
- **Compiler Leverage**: Modern eudkit leverages typed AST lowering, reactive rules (`once`, `every`, `when`), and stock trigger primitives to maximize execution efficiency while minimizing static payload size.
