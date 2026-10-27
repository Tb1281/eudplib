"""Scenario 15 (legacy) — read the local player's selected unit.

Intent: every frame, read the (non-synchronized) selected-unit pointer
at 0x6284B8; if a unit is selected, copy its hp / shield / unit id /
current order into display variables, warn when its hp drops below
25%, and notice when the selection's hp changes between frames.
Legacy idiom: f_dwread + CUnit.from_ptr (cached ptr->epd conversion).
(posX/posY are excluded on BOTH sides: the legacy members apply a
map-size mask that needs a loaded CHK, which the pilot harness does
not provide — see note.md.)
"""

from eudkit import *  # noqa: F403

SELECTED_UNIT_PTR = 0x6284B8

sel_hp = EUDVariable()
sel_shield = EUDVariable()
sel_unit_id = EUDVariable()
sel_order = EUDVariable()
prev_hp = EUDVariable()
has_selection = EUDVariable()


def read_selected_unit():
    global has_selection
    sel_ptr = f_dwread(SELECTED_UNIT_PTR)
    has_selection << 0

    if EUDIf()(sel_ptr != 0):
        has_selection << 1
        cu = CUnit.from_ptr(sel_ptr)
        sel_hp << cu.hp
        sel_shield << cu.shield
        sel_unit_id << cu.unitType
        sel_order << cu.order

        # Low-hp warning: hp < 25% of 1000 (this demo's full-hp scale).
        # Reporting = string-free deaths flag (pilot convention).
        if EUDIf()(cu.hp < 250):
            DoActions([SetDeaths(Player1, SetTo, 1, 202)])
        EUDEndIf()

        # Change notice: selection hp moved since last frame.
        if EUDIf()(sel_hp != prev_hp):
            prev_hp << sel_hp
            DoActions([SetDeaths(Player1, SetTo, 1, 203)])
        EUDEndIf()
    EUDEndIf()


def run():
    read_selected_unit()
    if EUDIf()(has_selection == 0):
        sel_hp << 0
        sel_shield << 0
        sel_unit_id << 0
        prev_hp << 0
    EUDEndIf()
