"""07-array-loop (legacy) — sum + in-place double over a 16-dword array.

Intent: every trigger loop, (1) sum all 16 elements of a dword
array into a total var, (2) double every element in place. Legacy
idiom: EUDArray backed by its own var block (72-byte stride per
element in the classic allocator); var-index get/set via
__getitem__/__setitem__, driven by an EUDWhile over a plain index
var.

Static vs executed: the count below is the static body cost; at run
time each loop body fires 16 times per trigger loop.
"""

from eudkit import *

# 16 dword slots; seeds 1..16 so the sum is live data, not dead code.
arr = EUDArray([i + 1 for i in range(16)])
total = EUDVariable(0)
idx = EUDVariable(0)
checksum = EUDVariable(0)


def run():
    global total, idx, checksum
    # Pass 1: accumulate.
    total << 0
    idx << 0
    if EUDWhile()(idx < 16):
        total << total + arr[idx]
        idx << idx + 1
    EUDEndWhile()

    # Pass 2: double in place.
    idx << 0
    if EUDWhile()(idx < 16):
        arr[idx] = arr[idx] * 2
        idx << idx + 1
    EUDEndWhile()

    # Keep the total live: fold it into a checksum var so a
    # downstream system could diff it.
    checksum << total + (total >> 16)
