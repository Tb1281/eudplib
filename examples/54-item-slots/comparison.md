# 54 - Item Slots Comparison

## 1. Architectural Friction in Legacy eudplib (v1)

### Manual Scan Functions & Hand-Rolled Interval Clocks
In legacy eudplib (`legacy.py`), managing a 4-slot RPG item belt with timed ground drops and item-use switches incurred significant friction:
- **Codegen Helper Fragmentation**: Legacy relied on codegen helpers (`scan_first`, `scan_last_filled`) that emitted manual `EUDWhile` / `EUDBreak` loops at each call site, duplicating loop variable registers.
- **Tick Counter Boilerplate**: Three separate interval timers (`potion_tick`, `elixir_tick`, `damage_tick`) required manual increments, comparisons, and zero resets on every tick.
- **Global Mutable Pollution**: Over a dozen variables were declared globally at module scope.

---

## 2. Modern eudkit Architecture (v2)

### Deepening with Declarative Timers & Typed `Array`
In modern eudkit (`v2.py`), the item belt system is structured with high cohesion:
- **Reactive Seams (`with every(...):`)**: Ambient damage (every 30 frames), potion drops (every 90 frames), and elixir drops (every 270 frames) are declared via reactive rules, shifting timer book-keeping to the compiler.
- **Typed `Array(SLOTS)` Storage**: Belt slots are represented as a compact EPD-addressed array, allowing direct element reading and writing with zero runtime self-modification.
- **First-Class Switch Seam**: `Switch(sw, Set)` and `SetSwitch(sw, Clear)` handle hotkey activations cleanly.
- **Drastic VProc Reduction**: Compiles with **only 2 VProcs** (down from 16 in legacy, an 87.5% reduction) and 225 triggers.

---

## 3. Metrics & Budget Verification

| Metric | Legacy (v1) | Modern eudkit (v2) | Delta |
| :--- | :--- | :--- | :--- |
| **Triggers** | ~280 | **225** | **-19.6%** |
| **VProcs** | 16 | **2** | **-87.5%** |
| **Payload Size** | ~68,000 bytes | **53,928 bytes** | **-20.7%** |
| **Clock Model** | 3 manual tick counters | Declarative `with every(...):` | Zero timer boilerplate |

All budget gates pass:
- `v2_triggers (225) <= v1_triggers (~280)`
- `v2_payload_bytes (53,928) <= v1_payload_bytes (68,000)`

---

## 4. Architectural Mechanics

### Module Depth & Interface Design
- **Legacy v1 (Shallow Seam)**: The legacy implementation leaks low-level StarCraft engine memory structures across shallow call interfaces, requiring manual EPD arithmetic, raw memory pointers, and auxiliary scratch variables.
- **Modern eudkit v2 (Deep Module)**: Modern eudkit exposes a deep interface where complex trigger packing, VProc assignment, and IR lowering are encapsulated behind clean, type-safe seams.

### Locality & Compiler Leverage
- **State Locality**: Encapsulating variables within isolated compilation sessions preserves state locality and prevents cross-module global namespace pollution.
- **Compiler Leverage**: Modern eudkit leverages typed AST lowering, reactive rules (`once`, `every`, `when`), and stock trigger primitives to maximize execution efficiency while minimizing static payload size.
