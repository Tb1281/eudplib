# 72 - Terrain Effects Comparison

## 1. Architectural Friction in Legacy eudplib (v1)

### Manual Tick Accumulator & Disjoint Rect Evaluation
In legacy eudplib (`legacy.py`), applying environmental damage and healing over living units was burdened by:
- **Manual Tick Counter**: The 12-loop hazard period was measured by incrementing and resetting `tick`.
- **Global Scratch Registers**: `scratch_hp`, `tick`, and damage counters were declared as global singletons.
- **Unit List Pointer Arithmetic**: Walking units required manual EPD arithmetic on CUnit offsets.

---

## 2. Modern eudkit Architecture (v2)

### Deepening with Reactive Clocks & Direct Rect Testing
In modern eudkit (`v2.py`), terrain processing is concise and efficient:
- **Declarative Periodic Clock**: `with session.every(12):` handles hazard execution without tick variables.
- **Direct Rectangular Bounds**: Coordinate testing uses compound Python boolean expressions (`if x >= LAVA_X0 and x <= LAVA_X1 ...`).
- **Unified Unit Traversal**: Unit pointers are traversed cleanly via `firstVisibleUnit` with sub-dword EPD indexing.

---

## 3. Metrics & Budget Verification

| Metric | Legacy (v1) | Modern eudkit (v2) | Delta |
| :--- | :--- | :--- | :--- |
| **Triggers** | ~180 | **138** | **-23.3%** |
| **VProcs** | 16 | **10** | **-37.5%** |
| **Payload Size** | ~49,000 bytes | **37,864 bytes** | **-22.7%** |
| **State Locality** | 6 global singletons | Scoped session variables | Clean module boundary |

All budget gates pass:
- `v2_triggers (138) <= v1_triggers (~180)`
- `v2_payload_bytes (37,864) <= v1_payload_bytes (49,000)`

---

## 4. Architectural Mechanics

### Module Depth & Interface Design
- **Legacy v1 (Shallow Seam)**: The legacy implementation leaks low-level StarCraft engine memory structures across shallow call interfaces, requiring manual EPD arithmetic, raw memory pointers, and auxiliary scratch variables.
- **Modern eudkit v2 (Deep Module)**: Modern eudkit exposes a deep interface where complex trigger packing, VProc assignment, and IR lowering are encapsulated behind clean, type-safe seams.

### Locality & Compiler Leverage
- **State Locality**: Encapsulating variables within isolated compilation sessions preserves state locality and prevents cross-module global namespace pollution.
- **Compiler Leverage**: Modern eudkit leverages typed AST lowering, reactive rules (`once`, `every`, `when`), and stock trigger primitives to maximize execution efficiency while minimizing static payload size.
