"""Scenario 18: Engine Unit List Census (Hidden Units) — Modern eudkit (v2).

Ideal Intent:
Census units across multiple engine lists (live units, hidden units, scanner sweeps):
1. Phase 1: Walk the active unit linked list (`0x628430`), increment `live_count`,
   and stamp each slot index with a generation token (`mark[slot] = mark_gen`).
2. Phase 2: Walk all 1700 CUnit slots. Valid slots (`unitType != 228`) whose generation mark
   does not match `mark_gen` are "hidden" units.
3. Count total hidden units and hidden Terran Marines.
4. Count Scanner Sweep units (`unitType == 33`).
5. Emit milestone death flag if any hidden unit is discovered.
"""

import eudkit
from eudkit import (
    session,
    Array,
    EPD,
    Player1,
    SetDeaths,
    SetTo,
    epdread_epd,
    wread_epd,
)


CUNIT_BASE_EPD = EPD(0x59CCA8)
STRIDE = 84
SCANNER_SWEEP = 33
MARINE = 0
NONE_TYPE = 228
MARK_BUF_EPD = EPD(0x6FE000)

live_count = session.var(0, name="live_count")
hidden_count = session.var(0, name="hidden_count")
hidden_marines = session.var(0, name="hidden_marines")
scanner_count = session.var(0, name="scanner_count")
mark_gen = session.var(0, name="mark_gen")
cur_epd = session.var(0, name="cur_epd")
slot = session.var(0, name="slot")
slot_idx = session.var(0, name="slot_idx")
slot_epd = session.var(0, name="slot_epd")
unit_type = session.var(0, name="unit_type")

# 1700-slot compact memory mark array (6,800 bytes, 0 VProcs)
mark = Array(1700, epd=MARK_BUF_EPD, session=session)

@session.fn
def game_loop():
    live_count = 0
    hidden_count = 0
    hidden_marines = 0
    scanner_count = 0
    mark_gen += 1

    # Phase 1: Walk active units and stamp generation mark
    cur_epd = epdread_epd(EPD(0x628430))
    while cur_epd != EPD(0):
        live_count += 1
        slot = (cur_epd - CUNIT_BASE_EPD) // STRIDE
        mark[slot] = mark_gen
        cur_epd = epdread_epd(cur_epd + 1)

    # Phase 2: Sweep table and find unmarked active units
    slot_idx = 0
    while slot_idx < 1700:
        slot_epd = CUNIT_BASE_EPD + slot_idx * STRIDE
        unit_type = wread_epd(slot_epd + 25, 0)
        if unit_type != NONE_TYPE:
            if unit_type == SCANNER_SWEEP:
                scanner_count += 1
            if mark[slot_idx] != mark_gen:
                hidden_count += 1
                if unit_type == MARINE:
                    hidden_marines += 1
        slot_idx += 1

    if hidden_count >= 1:
        SetDeaths(Player1, SetTo, 1, 205)
