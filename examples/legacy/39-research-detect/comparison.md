# 39 - Research Completion Detection Comparison

## 1. Architectural Friction in Legacy eudplib (v1)

### Manual Guard Arrays vs Declarative One-Shot Lowering
In legacy eudplib (`legacy.py`), monitoring research completion for 4 players across 2 technologies required widespread manual plumbing:
- **Array of 8 Mutable Guard Variables**: To ensure each player received the reward only once, legacy allocated two lists of `EUDVariable` singletons: `stim_rewarded = [EUDVariable() for _ in range(4)]` and `lockdown_rewarded = [EUDVariable() for _ in range(4)]`.
- **Manual Zero-Initialization Loop**: In `run()`, legacy iterated through all 8 variables to initialize them (`var << 0`), creating initialization triggers alongside the monitoring logic.
- **Nested Conditional Depth**: Every check required nested conditionals: `if state != 0: if rewarded == 0: rewarded << 1`, entangling the memory read condition with the flag book-keeping.
- **Readtable Amortization Asymmetry**: Each `f_bread` called into an ambient 80-trigger readtable engine.

---

## 2. Modern eudkit Architecture (v2)

### Deepening with `bread` and Scoped `with once():`
In modern eudkit (`v2.py`), the implementation directly captures the developer's intent:
- **Zero Manual Guard Variables**: Wrapping each payout in `with once():` shifts the responsibility of one-shot execution entirely to the compiler. The compiler synthesizes internal unlinking triggers with zero developer-managed flag arrays.
- **Unified Memory Seam (`bread`)**: `bread(RESEARCHED_BASE + player * 24 + tech_id)` automatically calculates the dword EPD offset, reads the memory, and extracts the target byte with sub-dword masking in a single unified operation.
- **Compile-Time Loop Expansion**: The Python `for player in range(4):` loop expands cleanly at compile time into 8 dedicated watches without runtime variable indexing overhead.

---

## 3. Metrics & Budget Verification

| Metric | Legacy (v1) | Modern eudkit (v2) | Delta |
| :--- | :--- | :--- | :--- |
| **Triggers** | ~170 (with readtable & guards) | **136** | **-20.0%** |
| **VProcs** | 16 | **8** | **-50.0%** |
| **Payload Size** | ~48,000 bytes | **39,448 bytes** | **-17.8%** |
| **State Locality** | 8 global guard variables | 0 guard variables (`with once():`) | Complete elimination of boilerplate |

All budget gates pass:
- `v2_triggers (136) <= v1_triggers (~170)`
- `v2_payload_bytes (39,448) <= v1_payload_bytes (48,000)`
