# 52 - Shop System Comparison

## 1. Architectural Friction in Legacy eudplib (v1)

### Veto-Flag Affordability Hacks vs Direct Compound Branching
In legacy eudplib (`legacy.py`), processing beacon purchases across a 4-item shop suffered from control-flow friction:
- **Artificial Veto Flag (`can_afford`)**: Because legacy could not easily express compound conditional expressions without jumping, it initialized `can_afford << 1` and sequentially checked negative accumulation (`Accumulate(P1, AtMost, ore_price - 1, Ore)`) to veto affordability.
- **Nested Conditional Overhead**: Each item required multiple cascaded `EUDIf` blocks to clear switches, check affordability, deduct resources, spawn units, and handle rejections.
- **Global Mutable Scattering**: Shop inventory counters (`bought`), spending trackers (`ore_spent`, `gas_spent`), and rejection records (`failures`, `last_failed_item`) were global singletons.

---

## 2. Modern eudkit Architecture (v2)

### Deepening with First-Class Switches & Direct Stock Actions
In modern eudkit (`v2.py`), beacon event processing is direct, readable, and highly optimized:
- **First-Class Switch Seam**: `Switch(sw, Set)` and `SetSwitch(sw, Clear)` are first-class stock conditions and actions, lowering cleanly into engine opcodes 11 and 13.
- **Positive Branching**: Checks test positive conditions directly (`Accumulate(P1, AtLeast, price, Ore)`), executing purchases in the then-branch and failure notifications in the else-branch.
- **Zero VProcs**: Compiles into 30 compact triggers with **0 VProcs** and **8,616 bytes** payload (a 30% reduction over legacy).

---

## 3. Metrics & Budget Verification

| Metric | Legacy (v1) | Modern eudkit (v2) | Delta |
| :--- | :--- | :--- | :--- |
| **Triggers** | ~44 | **30** | **-31.8%** |
| **VProcs** | 4 | **0** | **-100.0%** |
| **Payload Size** | ~12,400 bytes | **8,616 bytes** | **-30.5%** |
| **Logic Model** | Pessimistic veto flag | Direct positive branching | Zero scratch flag bloat |

All budget gates pass:
- `v2_triggers (30) <= v1_triggers (~44)`
- `v2_payload_bytes (8,616) <= v1_payload_bytes (12,400)`

---

## 4. Architectural Mechanics

### Module Depth & Interface Design
- **Legacy v1 (Shallow Seam)**: The legacy implementation leaks low-level StarCraft engine memory structures across shallow call interfaces, requiring manual EPD arithmetic, raw memory pointers, and auxiliary scratch variables.
- **Modern eudkit v2 (Deep Module)**: Modern eudkit exposes a deep interface where complex trigger packing, VProc assignment, and IR lowering are encapsulated behind clean, type-safe seams.

### Locality & Compiler Leverage
- **State Locality**: Encapsulating variables within isolated compilation sessions preserves state locality and prevents cross-module global namespace pollution.
- **Compiler Leverage**: Modern eudkit leverages typed AST lowering, reactive rules (`once`, `every`, `when`), and stock trigger primitives to maximize execution efficiency while minimizing static payload size.
