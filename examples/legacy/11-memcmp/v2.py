"""Scenario 11: Memory Compare (Buffer Equality) — Modern eudkit (v2).

Ideal Intent:
Compare two 4-dword (16-byte) configuration buffers (primary and shadow) for equality:
1. Check first half (dwords 0 and 1) and second half (dwords 2 and 3).
2. Raise `resync_flag = 1` if ANY dword differs between buffers.
3. Identify which half diverged for diagnostics (`diff_half = 1` for first half, `2` for second half, `0` if equal).
"""

import eudkit
from eudkit import (
    session,
    EPD,
    dwread_epd,
)


BUF_A_EPD = EPD(0x58A364 + 0x800)
BUF_B_EPD = EPD(0x58A364 + 0x840)

resync_flag = session.var(0, name="resync_flag")
diff_half = session.var(0, name="diff_half")
diff_first = session.var(0, name="diff_first")
diff_second = session.var(0, name="diff_second")

@session.fn
def game_loop():
    diff_first = 0
    diff_second = 0

    # First half compare (dwords 0 and 1)
    if dwread_epd(BUF_A_EPD + 0) != dwread_epd(BUF_B_EPD + 0):
        diff_first = 1
    if dwread_epd(BUF_A_EPD + 1) != dwread_epd(BUF_B_EPD + 1):
        diff_first = 1

    # Second half compare (dwords 2 and 3)
    if dwread_epd(BUF_A_EPD + 2) != dwread_epd(BUF_B_EPD + 2):
        diff_second = 1
    if dwread_epd(BUF_A_EPD + 3) != dwread_epd(BUF_B_EPD + 3):
        diff_second = 1

    # Overall equality verdict
    if diff_first != 0 or diff_second != 0:
        resync_flag = 1
    else:
        resync_flag = 0

    # Divergence localization
    if diff_first != 0:
        diff_half = 1
    elif diff_second != 0:
        diff_half = 2
    else:
        diff_half = 0
