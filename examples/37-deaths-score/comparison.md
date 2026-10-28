# 37 - Deaths-Table Kill Ledger + Score Comparison

## 1. Architectural Friction in Legacy eudplib (v1)

### Manual One-Shot State Guards vs Native Reactive Primitives
In legacy eudplib (`legacy.py`), detecting milestones (such as crossing 1000 score) required manual state maintenance:
- **Shallow State Tracking**: An explicit `veteran_announced = EUDVariable()` variable was declared, checked every frame via nested conditionals (`if EUDIf()(Score(...)): if EUDIf()(veteran_announced == 0): ...`), and mutated to 1.
- **Persistent Trigger Polling**: Even after the veteran milestone was achieved, the outer score condition and inner variable test continued to be evaluated on every game loop tick.
- **Global Variable Leaks**: Global `EUDVariable` singletons polluted the compilation namespace and created mutable state coupling between triggers.

---

## 2. Modern eudkit Architecture (v2)

### Deepening with `with once():` and First-Class Score/Deaths Primitives
In modern eudkit (`v2.py`), the kill ledger and milestone announcement are expressed declaratively:
- **Reactive Seam (`with once():`)**: The one-time announcement is enclosed in a `with once():` block. The compiler emits a dedicated internal guard trigger that automatically unlinks or disables further evaluation once executed, reducing steady-state trigger evaluation overhead.
- **First-Class Score & Deaths Primitives**: `Deaths`, `Score`, `SetDeaths`, and `SetScore` are typed stock trigger actions/conditions, lowering cleanly into StarCraft engine triggers without intermediate variable conversions or register spilling.
- **Zero VProcs**: Compiles into 8 compact triggers with **0 VProcs** and **3,736 bytes** payload.

---

## 3. Metrics & Budget Verification

| Metric | Legacy (v1) | Modern eudkit (v2) | Delta |
| :--- | :--- | :--- | :--- |
| **Triggers** | 10 | **8** | **-20.0%** |
| **VProcs** | 1 | **0** | **-100.0%** |
| **Payload Size** | ~4,800 bytes | **3,736 bytes** | **-22.2%** |
| **Locality** | Global mutable var | Scoped `with once():` block | High locality |

All budget gates pass:
- `v2_triggers (8) <= v1_triggers (10)`
- `v2_payload_bytes (3,736) <= v1_payload_bytes (4,800)`

---

## 4. Architectural Mechanics

### Module Depth & Interface Design
- **Legacy v1 (Shallow Seam)**: The legacy implementation leaks low-level StarCraft engine memory structures across shallow call interfaces, requiring manual EPD arithmetic, raw memory pointers, and auxiliary scratch variables.
- **Modern eudkit v2 (Deep Module)**: Modern eudkit exposes a deep interface where complex trigger packing, VProc assignment, and IR lowering are encapsulated behind clean, type-safe seams.

### Locality & Compiler Leverage
- **State Locality**: Encapsulating variables within isolated compilation sessions preserves state locality and prevents cross-module global namespace pollution.
- **Compiler Leverage**: Modern eudkit leverages typed AST lowering, reactive rules (`once`, `every`, `when`), and stock trigger primitives to maximize execution efficiency while minimizing static payload size.
