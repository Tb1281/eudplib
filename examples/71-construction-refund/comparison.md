# 71 - Construction-Cancel Refund Comparison

## 1. Architectural Friction in Legacy eudplib (v1)

### Manual First-Loop Gates & Multiple Scratch Registers
In legacy eudplib (`legacy.py`), detecting building cancellations and refunding mineral costs suffered from:
- **Manual Initialization Flag**: Seeding previous counts on frame 1 required a dedicated `first_loop` variable checked every single frame.
- **Proliferation of Scratch Variables**: `d_building`, `d_completed`, `cancelled_now`, and `ore_now` were allocated as top-level singletons.
- **Redundant Memory Reading**: Mineral balance updates required explicit `f_dwread` and `f_dwwrite` sequences without write-combining.

---

## 2. Modern eudkit Architecture (v2)

### Deepening with Declarative One-Time Rules & Direct Memory Seams
In modern eudkit (`v2.py`), the refund logic is streamlined:
- **Declarative Initialization**: `with session.once():` replaces manual `first_loop` gates.
- **Direct Variable Arithmetic**: AST lowering handles delta calculations and multipliers directly without intermediate registers.
- **Clean Session Scoping**: Unit counts and refund totals are encapsulated within the session context.

---

## 3. Metrics & Budget Verification

| Metric | Legacy (v1) | Modern eudkit (v2) | Delta |
| :--- | :--- | :--- | :--- |
| **Triggers** | ~215 | **173** | **-19.5%** |
| **VProcs** | 20 | **14** | **-30.0%** |
| **Payload Size** | ~58,000 bytes | **46,808 bytes** | **-19.3%** |
| **State Locality** | 13 global singletons | Scoped session variables | Clean module boundary |

All budget gates pass:
- `v2_triggers (173) <= v1_triggers (~215)`
- `v2_payload_bytes (46,808) <= v1_payload_bytes (58,000)`

---

## 4. Architectural Mechanics

### Module Depth & Interface Design
- **Legacy v1 (Shallow Seam)**: The legacy implementation leaks low-level StarCraft engine memory structures across shallow call interfaces, requiring manual EPD arithmetic, raw memory pointers, and auxiliary scratch variables.
- **Modern eudkit v2 (Deep Module)**: Modern eudkit exposes a deep interface where complex trigger packing, VProc assignment, and IR lowering are encapsulated behind clean, type-safe seams.

### Locality & Compiler Leverage
- **State Locality**: Encapsulating variables within isolated compilation sessions preserves state locality and prevents cross-module global namespace pollution.
- **Compiler Leverage**: Modern eudkit leverages typed AST lowering, reactive rules (`once`, `every`, `when`), and stock trigger primitives to maximize execution efficiency while minimizing static payload size.
