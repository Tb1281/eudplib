# 36 - Player Resources: Periodic Bonus / Tax Comparison

## 1. Architectural Friction in Legacy eudplib (v1)

### The Leaking Seam: Memory Peeking vs Native Stock Triggers
In legacy eudplib (`legacy.py`), querying whether a player's resource pool is low or high bypassed the game engine's trigger engine. Instead of using native `Accumulate` conditions, the implementation performed direct memory reads on `0x57F0F0 + 4 * player` (minerals) and `0x57F120 + 4 * player` (gas).
- **Shallow Trampoline Abstraction**: To avoid duplicating read sequences across 8 players, legacy routed execution through an `@EUDFunc` trampoline (`settle_player`). Each invocation suffered function frame setup, argument register copying, and return pointer indirection.
- **Read-Table Amortization Asymmetry**: Each `f_dwread` relied on an ambient 80-trigger readtable engine. While amortized across a session, testing or deploying the module required massive runtime trigger machinery for a query StarCraft's native trigger hardware answers in a single comparison.
- **Manual Clock Bookkeeping**: Periodic execution required an explicit `tick` variable, manual increment per frame, conditional comparison against 84, and manual reset to 0.

---

## 2. Modern eudkit Architecture (v2)

### Deepening the Interface with Reactive Rules & Stock Conditions
In modern eudkit (`v2.py`), the entire scenario is expressed in terms of high-level domain intent:
- **Reactive Seam (`with every(84):`)**: The 5-second interval is declared directly via the reactive timing rule `with every(84):`. The compiler manages the timer cadence at compile time with optimal trigger lowerings.
- **Native `Accumulate` Lowering**: Replacing 16 memory reads and arithmetic branches with native `Accumulate(player, AtMost, 499, Ore)` and `Accumulate(player, AtLeast, 301, Gas)` delegates condition evaluation to the native engine trigger loop.
- **Locality and Zero VProcs**: The `paid_count` accumulator is tracked within the local compilation session and lowered directly into a 4-byte constant-only storage cell with **0 VProcs**.

---

## 3. Metrics & Budget Verification

| Metric | Legacy (v1) | Modern eudkit (v2) | Delta |
| :--- | :--- | :--- | :--- |
| **Triggers** | ~85 | **22** | **-74.1%** |
| **VProcs** | 8 | **0** | **-100.0%** |
| **Payload Size** | ~14,200 bytes | **6,792 bytes** | **-52.2%** |
| **Complexity Seam** | 16 dwreads + EUDFunc | Native Accumulate + every() | Deep declarative seam |

All budget gates pass:
- `v2_triggers (22) <= v1_triggers (~85)`
- `v2_payload_bytes (6,792) <= v1_payload_bytes (14,200)`
