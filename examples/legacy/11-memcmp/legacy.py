"""11-memcmp (legacy) — compare two 4-dword buffers for equality.

Intent: a settings block (4 dwords at a fixed address) and its
shadow copy must match; every trigger loop, detect ANY difference
and raise a resync flag, and identify which half diverged for
logging. Legacy idiom: f_memcmp walks the buffers BYTE BY BYTE
through two EUDByteReaders inside an EUDFunc and returns the first
differing byte's difference.

Amortization note: the EUDFunc body and the byte readers' readtable
engine (~80T) exist once per session (import/library time) — this
delta counts only the call sites.
Static vs executed: an all-equal verdict EXECUTES the byte loop
over all 16 bytes (each readbyte runs the read chain); the static
call-site count hides that entirely.
"""

from eudkit import *

# Fixed scratch addresses (settings + shadow), 16 bytes each.
BUF_A = 0x58A364 + 0x800
BUF_B = 0x58A364 + 0x840

resync_flag = EUDVariable(0)
diff_half = EUDVariable(0)


def run():
    global resync_flag, diff_half
    # Whole-buffer verdict: f_memcmp returns 0 iff all 16 bytes match.
    if EUDIf()(f_memcmp(BUF_A, BUF_B, 16) != 0):
        resync_flag << 1
    if EUDElse()():
        resync_flag << 0
    EUDEndIf()

    # Which half diverged? Two 8-byte compares for the log path.
    if EUDIf()(f_memcmp(BUF_A, BUF_B, 8) != 0):
        diff_half << 1
    if EUDElseIf()(f_memcmp(BUF_A + 8, BUF_B + 8, 8) != 0):
        diff_half << 2
    if EUDElse()():
        diff_half << 0
    EUDEndIf()
