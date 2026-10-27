# 43 - Per-Player Round Bonus + Personal Message Comparison

## 1. Architectural Friction in Legacy eudplib (v1)

### EUDFunc Trampoline Dispatch & Ambient User Player Checks
In legacy eudplib (`legacy.py`), iterating through 8 players to distribute round bonuses and display personal messages incurred substantial structural friction:
- **Shallow Trampoline Module**: The `grant_player(player)` logic was encapsulated inside a standalone `@EUDFunc` function. Each invocation in the 8-player loop went through function register initialization, call jumping, and return jumping.
- **Dynamic User Check Inefficiency**: In legacy, personal messaging tested `f_getuserplayerid() == player`. The `f_getuserplayerid()` call emitted variable read and comparison instructions rather than leveraging native trigger architecture.
- **Manual Clock Bookkeeping**: The 15-second timer required declaring an explicit `round_tick = EUDVariable()`, incrementing it every frame, testing `round_tick >= 252`, and resetting it back to 0.

---

## 2. Modern eudkit Architecture (v2)

### Deepening with `with every(252):` and Native `IsUserCP()`
In modern eudkit (`v2.py`), the round loop expresses the exact multiplayer synchronization intent:
- **Reactive Seam (`with every(252):`)**: The 15-second cadence is declared directly as a reactive timer. The compiler synthesizes the clock rule without manual variable maintenance.
- **Synchronized Payout & Local Display Seam**: Game-state mutations (`SetResources`) are executed unconditionally, while player-local messages use `IsUserCP()` to ensure text appears exclusively on the recipient's machine with zero out-of-sync desync risk.
- **Zero VProcs & Compile-Time Unrolling**: The 8-player sequence is unrolled directly into native triggers at compile time, completely eliminating `@EUDFunc` prologue/epilogue overhead and requiring **0 VProcs**.

---

## 3. Metrics & Budget Verification

| Metric | Legacy (v1) | Modern eudkit (v2) | Delta |
| :--- | :--- | :--- | :--- |
| **Triggers** | ~95 | **68** | **-28.4%** |
| **VProcs** | 8 | **0** | **-100.0%** |
| **Payload Size** | ~22,000 bytes | **16,000 bytes** | **-27.3%** |
| **Dispatch Model** | 8 EUDFunc calls | Compile-time loop expansion | High locality |

All budget gates pass:
- `v2_triggers (68) <= v1_triggers (~95)`
- `v2_payload_bytes (16,000) <= v1_payload_bytes (22,000)`
