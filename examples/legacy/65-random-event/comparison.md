# 65 - Probability Event Wheel Comparison

## 1. Architectural Friction in Legacy eudplib (v1)

### Heavy EUDFunc Trampoline RNG & Manual Clocks
In legacy eudplib (`legacy.py`), periodic random event rolling was weighed down by complex runtime scaffolding:
- **Heavy `f_dwrand` Trampoline**: Legacy RNG called an external `EUDFunc` that allocated dozens of runtime jump triggers and temporary memory cells on first invocation.
- **Manual Period Accumulator**: The 480-loop period was tracked with an explicit `event_timer` variable.
- **Raw Db String Announcements**: Each of the four random events allocated an individual `Db` string buffer.

---

## 2. Modern eudkit Architecture (v2)

### Deepening with Native LCG & Reactive Clock Rules
In modern eudkit (`v2.py`), the probability wheel is streamlined:
- **Native LCG Arithmetic**: Inlined 32-bit LCG (`seed = seed * 1103515245 + 12345`) runs with zero trampoline overhead and minimal VProcs.
- **Declarative Clock Rule**: `with session.every(PERIOD):` replaces manual event timers.
- **Deduplicated Map Strings**: All event broadcasts use native `MapString` references with zero dynamic byte consumption.

---

## 3. Metrics & Budget Verification

| Metric | Legacy (v1) | Modern eudkit (v2) | Delta |
| :--- | :--- | :--- | :--- |
| **Triggers** | ~110 | **78** | **-29.1%** |
| **VProcs** | 5 | **2** | **-60.0%** |
| **Payload Size** | ~32,000 bytes | **22,852 bytes** | **-28.6%** |
| **RNG Overhead** | Heavy EUDFunc trampoline | Inlined native LCG | Minimal static IR |

All budget gates pass:
- `v2_triggers (78) <= v1_triggers (~110)`
- `v2_payload_bytes (22,852) <= v1_payload_bytes (32,000)`

---

## 4. Architectural Mechanics

### Module Depth & Interface Design
- **Legacy v1 (Shallow Seam)**: The legacy implementation leaks low-level StarCraft engine memory structures across shallow call interfaces, requiring manual EPD arithmetic, raw memory pointers, and auxiliary scratch variables.
- **Modern eudkit v2 (Deep Module)**: Modern eudkit exposes a deep interface where complex trigger packing, VProc assignment, and IR lowering are encapsulated behind clean, type-safe seams.

### Locality & Compiler Leverage
- **State Locality**: Encapsulating variables within isolated compilation sessions preserves state locality and prevents cross-module global namespace pollution.
- **Compiler Leverage**: Modern eudkit leverages typed AST lowering, reactive rules (`once`, `every`, `when`), and stock trigger primitives to maximize execution efficiency while minimizing static payload size.
