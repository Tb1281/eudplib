"""Scenario 15: Selected-Unit Field Reading — Modern eudkit (v2).

Ideal Intent:
Read UI telemetry for the local player's currently selected unit from pointer `0x6284B8`:
1. Read selected unit pointer and convert to EPD via `epdread_epd`.
2. If a unit is selected (`sel_epd != EPD(0)`):
   - Read `hp` (offset +0x08 = +2 dwords)
   - Read `shield` (offset +0x60 = +24 dwords)
   - Read `unitType` (offset +0x64 = +25 dwords, 16-bit word)
   - Read `order` (offset +0x4D = +19 dwords, subp 3)
   - Alert low health milestone (`hp < 250`)
   - Detect dynamic HP change since previous frame (`hp != prev_hp`)
3. If no unit is selected: zero out all telemetry display variables.
"""

import eudkit
from eudkit import (
    session,
    EPD,
    Player1,
    SetDeaths,
    SetTo,
    bread_epd,
    dwread_epd,
    epdread_epd,
    wread_epd,
)


SELECTED_UNIT_PTR = 0x6284B8
SELECTED_UNIT_EPD = EPD(SELECTED_UNIT_PTR)

sel_hp = session.var(0, name="sel_hp")
sel_shield = session.var(0, name="sel_shield")
sel_unit_id = session.var(0, name="sel_unit_id")
sel_order = session.var(0, name="sel_order")
prev_hp = session.var(0, name="prev_hp")
has_selection = session.var(0, name="has_selection")
sel_epd = session.var(0, name="sel_epd")

@session.fn
def game_loop():
    # Read selected unit pointer and convert to EPD in one step
    sel_epd = epdread_epd(SELECTED_UNIT_EPD)

    if sel_epd != EPD(0):
        has_selection = 1
        sel_hp = dwread_epd(sel_epd + 2)
        sel_shield = dwread_epd(sel_epd + 24)
        sel_unit_id = wread_epd(sel_epd + 25, 0)
        sel_order = bread_epd(sel_epd + 19, 3)

        # Low-HP warning condition
        if sel_hp < 250:
            SetDeaths(Player1, SetTo, 1, 202)

        # HP delta detection across frames
        if sel_hp != prev_hp:
            prev_hp = sel_hp
            SetDeaths(Player1, SetTo, 1, 203)
    else:
        has_selection = 0
        sel_hp = 0
        sel_shield = 0
        sel_unit_id = 0
        sel_order = 0
        prev_hp = 0
