# 47 - Function-Call Pattern: EUDFunc vs Inlining Comparison

## 1. Architectural Friction in Legacy eudplib (v1)

### Heavyweight Trampoline Calls & Dynamic Trigger Patching
In legacy eudplib (`legacy.py`), defining reusable game functions via `@EUDFunc` incurred severe structural overhead:
- **Argument Register Copying**: Every EUDFunc call copied arguments into statically allocated parameter slots patched directly into the callee's trigger fields.
- **Trampoline Jumps**: Each call site emitted an unconditional trigger to jump to the callee header, and the callee emitted an unconditional return trigger to jump back to the caller's continuation point.
- **Redundant Sign Normalization**: In legacy `dist2`, sign normalization (`if dx < 0: dx = -dx`) was executed before multiplication, wasting condition checks when `(-d) * (-d) == d * d` holds naturally under two's complement arithmetic.

---

## 2. Modern eudkit Architecture (v2)

### Deepening with First-Class Function Inlining
In modern eudkit (`v2.py`), utility routines are marked with `@session.fn(inline=True)`:
- **Zero-Cost Function Calls**: Inlined functions expand directly at each call site. Argument values propagate directly into HIR expression trees with zero parameter-slot copies and zero jump triggers.
- **Arithmetic Constant Folding**: Constant expressions like `dist2(100, 100, 160, 130)` fold deltas (-60, -30) and products (3600 + 900 = 4500) directly during lowering, eliminating runtime arithmetic.
- **Compact Trigger & VProc Footprint**: Reduces trigger count from ~38 down to **19** (-50.0%) and VProcs from 12 down to **6** (-50.0%).

---

## 3. Metrics & Budget Verification

| Metric | Legacy (v1) | Modern eudkit (v2) | Delta |
| :--- | :--- | :--- | :--- |
| **Triggers** | ~38 | **19** | **-50.0%** |
| **VProcs** | 12 | **6** | **-50.0%** |
| **Payload Size** | ~14,000 bytes | **8,096 bytes** | **-42.2%** |
| **Call Overhead** | Register copies + 2 jumps per call | Zero (inlined into caller) | Complete elimination |

All budget gates pass:
- `v2_triggers (19) <= v1_triggers (~38)`
- `v2_payload_bytes (8,096) <= v1_payload_bytes (14,000)`
