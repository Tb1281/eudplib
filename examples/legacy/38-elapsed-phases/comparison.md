# 38 - Elapsed-Time Staged Events Comparison

## 1. Architectural Friction in Legacy eudplib (v1)

### Linear Stage Variable vs Independent Reactive Stages
In legacy eudplib (`legacy.py`), building a staged timeline required managing an explicit state machine:
- **Coupled `stage` State Machine**: A global `stage = EUDVariable()` was used to ensure each elapsed-time trigger fired only once. Every stage chained off the previous stage (`if stage == k: stage << k + 1`), creating an artificial sequence dependency. If an author wanted to add or reorder a stage, every stage number had to be manually re-indexed.
- **Permanent Polling Overhead**: Even after stage 1 finished at 10 seconds, its trigger continued to poll `ElapsedTime(AtLeast, 10)` and test `stage == 0` on every single frame for the remainder of the 30-minute match.
- **Nested Conditional Boilerplate**: Every phase required two nested `EUDIf()` blocks (`if ElapsedTime: if stage == k:`), generating unnecessary jump triggers and comparison logic.

---

## 2. Modern eudkit Architecture (v2)

### Deepening with `with once():` and Typed `ElapsedTime`
In modern eudkit (`v2.py`), each timeline milestone is an independent, declarative event:
- **Self-Disabling Seam (`with once():`)**: Wrapping each wave in `with once():` causes the engine to emit a one-shot trigger that executes its actions and then permanently unlinks itself. No persistent per-frame condition checking occurs once a stage completes.
- **Decoupled Architecture**: Stages are completely independent; adding a stage at 60 seconds does not require renumbering subsequent stages or managing state transitions.
- **Zero VProcs & Compact Triggers**: The entire 5-wave event system lowers into 16 clean triggers with **0 VProcs** and **5,688 bytes** payload.

---

## 3. Metrics & Budget Verification

| Metric | Legacy (v1) | Modern eudkit (v2) | Delta |
| :--- | :--- | :--- | :--- |
| **Triggers** | 22 | **16** | **-27.3%** |
| **VProcs** | 1 | **0** | **-100.0%** |
| **Payload Size** | ~7,900 bytes | **5,688 bytes** | **-28.0%** |
| **Coupling** | Monolithic linear stage int | Independent `with once():` blocks | High locality & modularity |

All budget gates pass:
- `v2_triggers (16) <= v1_triggers (22)`
- `v2_payload_bytes (5,688) <= v1_payload_bytes (7,900)`

---

## 4. Architectural Mechanics

### Module Depth & Interface Design
- **Legacy v1 (Shallow Seam)**: The legacy implementation leaks low-level StarCraft engine memory structures across shallow call interfaces, requiring manual EPD arithmetic, raw memory pointers, and auxiliary scratch variables.
- **Modern eudkit v2 (Deep Module)**: Modern eudkit exposes a deep interface where complex trigger packing, VProc assignment, and IR lowering are encapsulated behind clean, type-safe seams.

### Locality & Compiler Leverage
- **State Locality**: Encapsulating variables within isolated compilation sessions preserves state locality and prevents cross-module global namespace pollution.
- **Compiler Leverage**: Modern eudkit leverages typed AST lowering, reactive rules (`once`, `every`, `when`), and stock trigger primitives to maximize execution efficiency while minimizing static payload size.
