"""10-memset (legacy) — clear a 64-dword scratch buffer on a flag.

Intent: a wave system owns a 64-dword scratch region. Whenever the
clear_flag var is raised, zero the whole region exactly once and
drop the flag. Legacy idiom: an EUDWhile counting loop with
f_dwwrite_epd per dword — every iteration re-enters the generic
write path (address recompute per element + write triggers).

Static vs executed: the static count is the loop body once; at run
time the body fires 64 times per clear.
"""

from eudkit import *

# Scratch region base: a Db blob standing in for the wave system's
# fixed scratch table (its EPD is fixed at build time).
scratch = Db(b"\x00" * (64 * 4))
scratch_epd = EPD(scratch)

clear_flag = EUDVariable(0)
idx = EUDVariable(0)
clears_done = EUDVariable(0)


def run():
    global clear_flag, idx, clears_done
    if EUDIf()(clear_flag != 0):
        # Hand-written per-dword loop: idx walks the region.
        idx << 0
        if EUDWhile()(idx < 64):
            f_dwwrite_epd(scratch_epd + idx, 0)
            idx << idx + 1
        EUDEndWhile()

        clear_flag << 0
        clears_done << clears_done + 1
    EUDEndIf()
