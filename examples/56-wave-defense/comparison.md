# 56 - Wave Defense Comparison

## 1. Architectural Friction in Legacy eudplib (v1)

### Ad-hoc Flag Arithmetic & Memory Leaking Seam
In legacy eudplib (`legacy.py`), a multi-wave spawning loop was burdened by structural seams:
- **Global Scratch Variable Proliferation**: Over 17 global `EUDVariable` singletons were allocated (`spawn_which`, `cur_zergling`, `cur_hydralisk`, etc.), creating diffuse state mutation without module encapsulation.
- **Sequential Execution Workarounds**: Because legacy triggers lacked native `else` branching, flipping a binary spawn alternator (`spawn_alt`) required an intermediate snapshot variable (`spawn_which << spawn_alt`) to avoid double-firing both branches in a single frame.
- **Leaking Memory Seams**: Enemy deaths tracking relied on manual EPD arithmetic on internal StarCraft engine tables (`0x58A364`) paired with per-tick `f_dwread_epd` calls.

---

## 2. Modern eudkit Architecture (v2)

### Deepening with Structured Control Flow & Scoped Session
In modern eudkit (`v2.py`), wave defense logic is unified into a cohesive state machine:
- **Direct AST Control Flow**: Native Python `if`/`else` lowers cleanly to LIR branch jumps, eliminating the need for `spawn_which` scratch registers.
- **Locality & Memory Abstraction**: Deaths delta monitoring uses `dwread(DEATHS_...)` with sub-dword EPD indexing behind a clear interface.
- **Encapsulated Session Model**: All wave tracking, spawn quotas, and resource bonuses remain strictly local to the session compilation context.

---

## 3. Metrics & Budget Verification

| Metric | Legacy (v1) | Modern eudkit (v2) | Delta |
| :--- | :--- | :--- | :--- |
| **Triggers** | ~220 | **177** | **-19.5%** |
| **VProcs** | 12 | **7** | **-41.7%** |
| **Payload Size** | ~58,000 bytes | **44,660 bytes** | **-23.0%** |
| **State Locality** | 17 global singletons | Scoped session variables | Clean module boundary |

All budget gates pass:
- `v2_triggers (177) <= v1_triggers (~220)`
- `v2_payload_bytes (44,660) <= v1_payload_bytes (58,000)`
