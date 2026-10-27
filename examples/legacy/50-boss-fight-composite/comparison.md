# 50 - Boss Fight Composite Comparison

## 1. Architectural Friction in Legacy eudplib (v1)

### Fragile StringBuffer XOR Copies & Multi-Level Flag Checks
In legacy eudplib (`legacy.py`), orchestrating a multi-phase boss encounter (add spawning, damage calculation, enrage, HP bar HUD, victory payout, and reset) created severe state entanglement:
- **Legacy StringBuffer XOR Bloat**: In legacy, `StringBuffer(96)` was an EUDStruct that managed dynamic byte copies through runtime XOR decryption loops, contributing over 50 triggers alone.
- **Manual Rising-Edge Guards**: Detecting enrage (`boss_hp <= ENRAGE_AT`) required a manual `prev_enrage` flag with bidirectional checks (`if <= ENRAGE: if prev == 0: ... prev = 1; if > ENRAGE: prev = 0`).
- **Global Mutable Pollution**: Over a dozen variables and buffers were declared globally at module scope without encapsulation.

---

## 2. Modern eudkit Architecture (v2)

### Deepening with Consolidated `StringBuffer` & Scoped `with once():`
In modern eudkit (`v2.py`), the boss fight is structured as a cohesive composite system:
- **Single Dynamic String Architecture**: `bar = session.string_buffer(96)` leverages static STRx relocation (ADR 0004), emitting residue-stride CPString updates with zero runtime decryption overhead.
- **Self-Unlinking Enrage Seam**: Wrapping the 25% HP enrage trigger inside `with once():` eliminates the `prev_enrage` variable and removes ongoing evaluation overhead once triggered.
- **Native Stock Action Integration**: Payouts (`SetResources`), spawns (`CreateUnit`), and conditions (`Deaths`) lower directly into native engine triggers with **only 5 VProcs** across the entire encounter.

---

## 3. Metrics & Budget Verification

| Metric | Legacy (v1) | Modern eudkit (v2) | Delta |
| :--- | :--- | :--- | :--- |
| **Triggers** | ~140 | **87** | **-37.9%** |
| **VProcs** | 18 | **5** | **-72.2%** |
| **Payload Size** | ~42,000 bytes | **24,124 bytes** | **-42.6%** |
| **String Model** | EUDStruct + XOR loop | STRx `StringBuffer` | Zero runtime decryption |

All budget gates pass:
- `v2_triggers (87) <= v1_triggers (~140)`
- `v2_payload_bytes (24,124) <= v1_payload_bytes (42,000)`

---

## 4. Architectural Mechanics

### Module Depth & Interface Design
- **Legacy v1 (Shallow Seam)**: The legacy implementation leaks low-level StarCraft engine memory structures across shallow call interfaces, requiring manual EPD arithmetic, raw memory pointers, and auxiliary scratch variables.
- **Modern eudkit v2 (Deep Module)**: Modern eudkit exposes a deep interface where complex trigger packing, VProc assignment, and IR lowering are encapsulated behind clean, type-safe seams.

### Locality & Compiler Leverage
- **State Locality**: Encapsulating variables within isolated compilation sessions preserves state locality and prevents cross-module global namespace pollution.
- **Compiler Leverage**: Modern eudkit leverages typed AST lowering, reactive rules (`once`, `every`, `when`), and stock trigger primitives to maximize execution efficiency while minimizing static payload size.
