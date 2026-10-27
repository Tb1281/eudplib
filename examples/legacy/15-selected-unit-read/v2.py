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

from pathlib import Path
import eudkit
from eudkit import (
    EPD,
    Player1,
    SetDeaths,
    SetTo,
    bread_epd,
    dwread_epd,
    epdread_epd,
    wread_epd,
)

REPO_ROOT = Path(__file__).resolve().parents[3]
BASEMAP_PATH = REPO_ROOT / "tests" / "outputmap" / "basemap" / "basemap_strx.scx"

SELECTED_UNIT_PTR = 0x6284B8
SELECTED_UNIT_EPD = EPD(SELECTED_UNIT_PTR)


def build(session: eudkit.Session):
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

    return {
        "sel_hp": sel_hp,
        "sel_shield": sel_shield,
        "sel_unit_id": sel_unit_id,
        "sel_order": sel_order,
        "prev_hp": prev_hp,
        "has_selection": has_selection,
    }


def main():
    session = eudkit.Session(BASEMAP_PATH)
    build(session)
    out_path = Path(__file__).parent / "output_v2.scx"
    report = session.compile(out_path)
    print("Scenario 15 (v2) compiled successfully:")
    print(f"  Triggers: {report.trigger_count}")
    print(f"  Payload:  {report.payload_size} bytes")
    print(f"  VProcs:   {report.vproc_count}")
    return report


if __name__ == "__main__":
    main()
