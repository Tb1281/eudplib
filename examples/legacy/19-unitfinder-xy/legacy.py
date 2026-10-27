"""Scenario 19 (legacy) — find units near a target via the Unit Finder.

Intent: every frame, find all units whose position is within
(target +/- radius) on both axes, using the engine's sorted Unit
Finder X array (0x66FF78: 3400 entries of {dword x, dword unit ptr},
ascending by x). Count matches and latch the first match's pointer.
Neither stack has a finder API — both sides hand-roll the walk over
the raw arrays (entry layout per edac/142561; identical on both
sides, so the comparison is layout-neutral).
Legacy idiom: EUDWhile + f_dwread_epd cursor reads.
"""

from eudkit import *  # noqa: F403

UNIT_FINDER_X = 0x66FF78
FINDER_ENTRIES = 3400
FINDER_BASE_EPD = EPD(UNIT_FINDER_X)
POS_Y_OFFSET = 0x2A  # posY word inside the CUnit record

target_x = EUDVariable()  # set by other code (mouse / spell target)
target_y = EUDVariable()
radius = EUDVariable()
found_count = EUDVariable()
first_found_ptr = EUDVariable()


def run():
    global found_count, first_found_ptr
    found_count << 0
    first_found_ptr << 0
    lo_x = target_x - radius
    hi_x = target_x + radius
    lo_y = target_y - radius
    hi_y = target_y + radius

    idx = EUDVariable()
    idx << 0
    EUDWhile()(idx < FINDER_ENTRIES)
    # Entry = 2 dwords: [0] = x coordinate, [1] = unit pointer.
    coord = f_dwread_epd(FINDER_BASE_EPD + idx * 2)
    if EUDIf()(coord > hi_x):
        EUDBreak()  # sorted ascending: nothing further can match
    EUDEndIf()
    if EUDIf()(coord >= lo_x):
        unit_ptr = f_dwread_epd(FINDER_BASE_EPD + idx * 2 + 1)
        pos_y = f_wread(unit_ptr + POS_Y_OFFSET)
        if EUDIf()(pos_y >= lo_y):
            if EUDIf()(pos_y <= hi_y):
                if EUDIf()(found_count == 0):
                    first_found_ptr << unit_ptr
                EUDEndIf()
                found_count += 1
            EUDEndIf()
        EUDEndIf()
    EUDEndIf()
    idx += 1
    EUDEndWhile()
