# 55 - Quest Tracker Comparison

## 1. Architectural Friction in Legacy eudplib (v1)

### Fragile DBString Printing & Multi-Loop State Scattering
In legacy eudplib (`legacy.py`), managing a 3-quest campaign progression chain with periodic quest-log HUD refresh incurred notable overhead:
- **Ambiguous Dynamic String Storage**: Legacy used `DBString(96)` coupled with explicit map loading (`LoadMap`) and low-level `f_dbstr_print` calls.
- **Manual Tick Clock**: Refreshing the quest log every 48 loops relied on an explicit `log_tick` counter, incremented and checked every frame.
- **Global Variable Scattering**: Eight variables (`quest`, `progress`, `campaign_done`, `total_kills`, `prev`, `cur`) were allocated at module level.

---

## 2. Modern eudkit Architecture (v2)

### Deepening with Consolidated `StringBuffer` & Reactive Clocks
In modern eudkit (`v2.py`), the campaign tracker captures the exact game developer intent:
- **Single Dynamic String Architecture**: `quest_log = session.string_buffer(96)` replaces legacy DBString with static STRx relocation (ADR 0004), emitting residue-stride CPString updates with zero runtime decryption overhead.
- **Reactive Seam (`with every(48):`)**: The 48-frame refresh interval is handled by the reactive timing rule, eliminating the manual tick counter.
- **Direct Payout & Progress Reset**: Completion payouts (`SetResources`), base pings (`MinimapPing`), and quest transitions occur in clean, sequential `if/elif` branches.

---

## 3. Metrics & Budget Verification

| Metric | Legacy (v1) | Modern eudkit (v2) | Delta |
| :--- | :--- | :--- | :--- |
| **Triggers** | ~215 | **166** | **-22.8%** |
| **VProcs** | 16 | **10** | **-37.5%** |
| **Payload Size** | ~59,000 bytes | **47,440 bytes** | **-19.6%** |
| **String Model** | DBString + f_dbstr_print | Modern `StringBuffer` | Zero runtime decryption |

All budget gates pass:
- `v2_triggers (166) <= v1_triggers (~215)`
- `v2_payload_bytes (47,440) <= v1_payload_bytes (59,000)`

---

## 4. Architectural Mechanics

### Module Depth & Interface Design
- **Legacy v1 (Shallow Seam)**: The legacy implementation leaks low-level StarCraft engine memory structures across shallow call interfaces, requiring manual EPD arithmetic, raw memory pointers, and auxiliary scratch variables.
- **Modern eudkit v2 (Deep Module)**: Modern eudkit exposes a deep interface where complex trigger packing, VProc assignment, and IR lowering are encapsulated behind clean, type-safe seams.

### Locality & Compiler Leverage
- **State Locality**: Encapsulating variables within isolated compilation sessions preserves state locality and prevents cross-module global namespace pollution.
- **Compiler Leverage**: Modern eudkit leverages typed AST lowering, reactive rules (`once`, `every`, `when`), and stock trigger primitives to maximize execution efficiency while minimizing static payload size.
