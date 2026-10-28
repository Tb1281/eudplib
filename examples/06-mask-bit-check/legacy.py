"""06-mask-bit-check (legacy) — arbitrary-mask bit tests on a flags dword.

Intent: a flags dword at 0x58A364+0x200 encodes ability states; every
trigger loop, test three individual bits (0x00000001 "shield ready",
0x00000020 "stealth active", 0x00000400 "hero aura") and drive one
lamp var per bit, plus a combined "any of the three set" branch.
Legacy idiom: one f_dwread_epd materializes the dword, then plain
&-tests feed EUDIf gates — the engine has no arbitrary-mask
condition, so a read is unavoidable on both stacks.

Amortization note: f_dwread_epd rides the import-time readtable
engine (~80T, built once per session, excluded from this delta).
"""

from eudkit import *

FLAGS_EPD = EPD(0x58A364 + 0x200)

MASK_SHIELD = 0x00000001
MASK_STEALTH = 0x00000020
MASK_AURA = 0x00000400

lamp_shield = EUDVariable(0)
lamp_stealth = EUDVariable(0)
lamp_aura = EUDVariable(0)
lamp_any = EUDVariable(0)


def run():
    global lamp_shield, lamp_stealth, lamp_aura, lamp_any
    # One read; the three tests share it (cheapest-path pattern
    # inside the legacy API: materialize once, test many).
    flags = f_dwread_epd(FLAGS_EPD)

    if EUDIf()(flags & MASK_SHIELD != 0):
        lamp_shield << 1
    if EUDElse()():
        lamp_shield << 0
    EUDEndIf()

    if EUDIf()(flags & MASK_STEALTH != 0):
        lamp_stealth << 1
    if EUDElse()():
        lamp_stealth << 0
    EUDEndIf()

    if EUDIf()(flags & MASK_AURA != 0):
        lamp_aura << 1
    if EUDElse()():
        lamp_aura << 0
    EUDEndIf()

    # Combined test: any of the three bits set.
    if EUDIf()(flags & (MASK_SHIELD | MASK_STEALTH | MASK_AURA) != 0):
        lamp_any << 1
    if EUDElse()():
        lamp_any << 0
    EUDEndIf()
