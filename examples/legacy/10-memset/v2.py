"""Scenario 10: Memset (Buffer Clear) — Modern eudkit (v2).

Ideal Intent:
Clear a 64-dword scratch region on demand:
1. Detect request flag (`clear_flag != 0`).
2. Iterate through 64 dwords, setting each element to 0 via direct memory write.
3. Reset request flag to 0 and increment completion counter (`clears_done += 1`).
"""

import eudkit
from eudkit import (
    session,
    EPD,
    dwwrite_epd,
)


SCRATCH_EPD = EPD(0x6FE000)

clear_flag = session.var(0, name="clear_flag")
idx = session.var(0, name="idx")
clears_done = session.var(0, name="clears_done")

@session.fn
def game_loop():
    if clear_flag != 0:
        idx = 0
        while idx < 64:
            dwwrite_epd(SCRATCH_EPD + idx, 0)
            idx += 1

        clear_flag = 0
        clears_done += 1
