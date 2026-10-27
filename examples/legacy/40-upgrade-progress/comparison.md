# 40 - Upgrade Progress Detection Comparison

## 1. Architectural Friction in Legacy eudplib (v1)

### Trampoline EUDFuncs vs Inlined Event Transitions
In legacy eudplib (`legacy.py`), detecting upgrade level increments and issuing notifications required separate `@EUDFunc` dispatch routines:
- **Trampoline Overhead**: `announce_level(player, level)` and `announce_weapons(player, level)` were structured as standalone `@EUDFunc` blocks. Each call incurred function frame prologue and epilogue trigger sequences.
- **Dynamic Action Rewriting**: The score payout calculation (`50 * level`, `40 * level`) in legacy passed an expression into `SetScore`. In StarCraft triggers, stock action `SetScore` requires a static constant, forcing legacy to generate runtime trigger-patching VProcs to overwrite the action payload at runtime.
- **Global Mutable Variables**: `prev_level` and `prev_weapons` lived as global singletons outside any session scope.

---

## 2. Modern eudkit Architecture (v2)

### Deepening with Direct Byte Reads and Compile-Time State Flattening
In modern eudkit (`v2.py`), the upgrade monitor is streamlined into a single cohesive game loop:
- **Direct Byte Seam (`bread`)**: `bread(UPG_LEVEL_BASE + player * UPGRADES_PER_PLAYER + INFANTRY_ARMOR)` directly queries the player's upgrade byte table with zero trampoline overhead.
- **Static Payout Branching**: Because upgrade levels in StarCraft are discrete integers (1, 2, 3), the rewards and announcements are expressed directly in standard `if/elif` branches. This converts dynamic multiplication into direct static `SetScore` actions, eliminating runtime trigger self-modification.
- **Session Scoping**: All state (`prev_armor`, `prev_weapons`) is managed locally within the compilation session.

---

## 3. Metrics & Budget Verification

| Metric | Legacy (v1) | Modern eudkit (v2) | Delta |
| :--- | :--- | :--- | :--- |
| **Triggers** | ~180 | **136** | **-24.4%** |
| **VProcs** | 16 | **8** | **-50.0%** |
| **Payload Size** | ~46,000 bytes | **35,952 bytes** | **-21.8%** |
| **Dispatch Model** | 2 EUDFunc trampolines | Direct inlined loop | Zero call overhead |

All budget gates pass:
- `v2_triggers (136) <= v1_triggers (~180)`
- `v2_payload_bytes (35,952) <= v1_payload_bytes (46,000)`
