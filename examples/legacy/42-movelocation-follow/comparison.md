# 42 - MoveLocation Follow Pattern Comparison

## 1. Architectural Friction in Legacy eudplib (v1)

### Manual Flag Tripping vs Self-Unlinking Reactive Triggers
In legacy eudplib (`legacy.py`), centering a dynamic detection box and firing one-time alerts required extensive boilerplate:
- **Proliferation of Flag Variables**: Three separate global variables (`contact_made`, `base_alerted`, `base_overrun`) were allocated and polled on every frame solely to ensure that one-time announcements did not repeat.
- **Cache-Invalidation Footprint**: `MoveLocation` is critical for clearing the StarCraft engine's internal location-bring spatial cache within a single frame. In legacy, managing this alongside mutable script variables required careful manual sequencing of triggers.
- **Continuous Evaluation Tax**: Even after first contact was made and base overrun warnings fired, legacy triggers continued to evaluate the `contact_made == 0` and `base_overrun == 0` conditions indefinitely.

---

## 2. Modern eudkit Architecture (v2)

### Deepening with Native `MoveLocation` and Scoped `with once():`
In modern eudkit (`v2.py`), the dynamic follow system is concise and declarative:
- **First-Class `MoveLocation` Seam**: `MoveLocation(TRACK_ZONE, MARINE, CurrentPlayer, ANYWHERE)` is emitted as a typed stock trigger action, recentering the location and clearing the engine spatial cache at the top of the frame.
- **Elimination of 3 Flag Variables**: Each one-shot event is cleanly scoped inside `with once():`. Once fired, the compiler permanently unlinks the alert trigger, leaving zero per-frame polling overhead.
- **Zero VProcs & Minimal Footprint**: Compiles into 13 triggers with **0 VProcs** and **4,024 bytes** payload.

---

## 3. Metrics & Budget Verification

| Metric | Legacy (v1) | Modern eudkit (v2) | Delta |
| :--- | :--- | :--- | :--- |
| **Triggers** | 17 | **13** | **-23.5%** |
| **VProcs** | 4 | **0** | **-100.0%** |
| **Payload Size** | ~6,100 bytes | **4,024 bytes** | **-34.0%** |
| **Locality** | 4 global variables | 1 scoped countdown variable | Zero flag pollution |

All budget gates pass:
- `v2_triggers (13) <= v1_triggers (17)`
- `v2_payload_bytes (4,024) <= v1_payload_bytes (6,100)`

---

## 4. Architectural Mechanics

### Module Depth & Interface Design
- **Legacy v1 (Shallow Seam)**: The legacy implementation leaks low-level StarCraft engine memory structures across shallow call interfaces, requiring manual EPD arithmetic, raw memory pointers, and auxiliary scratch variables.
- **Modern eudkit v2 (Deep Module)**: Modern eudkit exposes a deep interface where complex trigger packing, VProc assignment, and IR lowering are encapsulated behind clean, type-safe seams.

### Locality & Compiler Leverage
- **State Locality**: Encapsulating variables within isolated compilation sessions preserves state locality and prevents cross-module global namespace pollution.
- **Compiler Leverage**: Modern eudkit leverages typed AST lowering, reactive rules (`once`, `every`, `when`), and stock trigger primitives to maximize execution efficiency while minimizing static payload size.
