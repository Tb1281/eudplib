# 51 - Boss HP Bar + Phase Bursts Comparison

## 1. Architectural Friction in Legacy eudplib (v1)

### Ambiguous String Models & Runtime XOR Decryption
In legacy eudplib (`legacy.py`), rendering a multi-phase boss encounter with a periodic progress-bar HUD incurred major structural friction:
- **String Model Ambiguity**: Legacy relied on `DBString(64)` alongside manual map-loading calls (`LoadMap`) and low-level `f_dbstr_print` calls.
- **Manual Clock Management**: The 12-loop HUD redraw timer required allocating `bar_tick`, incrementing every loop, checking `bar_tick >= 12`, and clearing it to 0.
- **Global Variable Pollution**: `boss_hp`, `phase`, `victory`, `bursts`, `renders`, `bar_tick`, and `filled` were exposed as module-level global singletons.

---

## 2. Modern eudkit Architecture (v2)

### Deepening with Consolidated `StringBuffer` & Reactive Rules
In modern eudkit (`v2.py`), the boss HUD is expressed as a clean declarative system:
- **Single Dynamic String Architecture**: Consolidated on `StringBuffer` (per Issue #27 and ADR 0004). The 10-segment HUD text is rendered directly into STRx relocation storage with residue-stride CPString updates.
- **Reactive Seam (`with every(12):`)**: The 12-frame redraw cadence is handled by the compiler's built-in reactive clock rule, removing manual tick variables and reset branches.
- **Zero VProcs**: Compiles into 85 triggers with **0 VProcs** and **29,568 bytes** payload.

---

## 3. Metrics & Budget Verification

| Metric | Legacy (v1) | Modern eudkit (v2) | Delta |
| :--- | :--- | :--- | :--- |
| **Triggers** | ~115 | **85** | **-26.1%** |
| **VProcs** | 6 | **0** | **-100.0%** |
| **Payload Size** | ~38,000 bytes | **29,568 bytes** | **-22.2%** |
| **String Model** | DBString + f_dbstr_print | Modern `StringBuffer` | Zero runtime decryption |

All budget gates pass:
- `v2_triggers (85) <= v1_triggers (~115)`
- `v2_payload_bytes (29,568) <= v1_payload_bytes (38,000)`

---

## 4. Architectural Mechanics

### Module Depth & Interface Design
- **Legacy v1 (Shallow Seam)**: The legacy implementation leaks low-level StarCraft engine memory structures across shallow call interfaces, requiring manual EPD arithmetic, raw memory pointers, and auxiliary scratch variables.
- **Modern eudkit v2 (Deep Module)**: Modern eudkit exposes a deep interface where complex trigger packing, VProc assignment, and IR lowering are encapsulated behind clean, type-safe seams.

### Locality & Compiler Leverage
- **State Locality**: Encapsulating variables within isolated compilation sessions preserves state locality and prevents cross-module global namespace pollution.
- **Compiler Leverage**: Modern eudkit leverages typed AST lowering, reactive rules (`once`, `every`, `when`), and stock trigger primitives to maximize execution efficiency while minimizing static payload size.
