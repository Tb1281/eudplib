"""Scenario 07: Array Loop (Sum and In-Place Double) — Modern eudkit (v2).

Ideal Intent:
Process a 16-dword array in two sequential loops:
1. Pass 1: Sum all 16 array elements into a running total variable.
2. Pass 2: In-place double every element (`arr[idx] = arr[idx] * 2`).
3. Compute a final 32-bit checksum (`total + (total >> 16)`).
"""

import eudkit
from eudkit import (
    session,
    Array,
    EPD,
)


ARR_EPD = EPD(0x6FE000)

# 16-element typed array allocated in compact 4-byte memory space
arr = Array([i + 1 for i in range(16)], epd=ARR_EPD, session=session)
total = session.var(0, name="total")
idx = session.var(0, name="idx")
checksum = session.var(0, name="checksum")

@session.fn
def game_loop():
    # Pass 1: Accumulate elements
    total = 0
    idx = 0
    while idx < 16:
        total += arr[idx]
        idx += 1

    # Pass 2: Double elements in place
    idx = 0
    while idx < 16:
        arr[idx] = arr[idx] * 2
        idx += 1

    # Checksum calculation
    checksum = total + (total >> 16)
