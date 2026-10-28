/**
 * Scenario 15: Selected-Unit Field Reading — Modern eudkit (v2).
 *
 * Ideal Intent:
 * Read UI telemetry for the local player's currently selected unit from pointer `0x6284B8`:
 * 1. Read selected unit pointer and convert to EPD via `epdread_epd`.
 * 2. If a unit is selected (`sel_epd != EPD(0)`):
 *    - Read `hp` (offset +0x08 = +2 dwords)
 *    - Read `shield` (offset +0x60 = +24 dwords)
 *    - Read `unitType` (offset +0x64 = +25 dwords, 16-bit word)
 *    - Read `order` (offset +0x4D = +19 dwords, subp 3)
 *    - Alert low health milestone (`hp < 250`)
 *    - Detect dynamic HP change since previous frame (`hp != prev_hp`)
 * 3. If no unit is selected: zero out all telemetry display variables.
 */

import {
    EPD,
    Player1,
    SetDeaths,
    SetTo,
    bread_epd,
    dwread_epd,
    epdread_epd,
    session,
    wread_epd,
} from "eudkit";

export const SELECTED_UNIT_PTR = 0x6284B8;
export const SELECTED_UNIT_EPD = EPD(SELECTED_UNIT_PTR);
export let sel_hp = session.var(0, "sel_hp");
export let sel_shield = session.var(0, "sel_shield");
export let sel_unit_id = session.var(0, "sel_unit_id");
export let sel_order = session.var(0, "sel_order");
export let prev_hp = session.var(0, "prev_hp");
export let has_selection = session.var(0, "has_selection");
export let sel_epd = session.var(0, "sel_epd");

export function main(): void {
    sel_epd = epdread_epd(SELECTED_UNIT_EPD);
    if ((sel_epd != EPD(0))) {
        has_selection = 1;
        sel_hp = dwread_epd((sel_epd + 2));
        sel_shield = dwread_epd((sel_epd + 24));
        sel_unit_id = wread_epd((sel_epd + 25), 0);
        sel_order = bread_epd((sel_epd + 19), 3);
        if ((sel_hp < 250)) {
            SetDeaths(Player1, SetTo, 1, 202);
        }
        if ((sel_hp != prev_hp)) {
            prev_hp = sel_hp;
            SetDeaths(Player1, SetTo, 1, 203);
        }
    } else {
        has_selection = 0;
        sel_hp = 0;
        sel_shield = 0;
        sel_unit_id = 0;
        sel_order = 0;
        prev_hp = 0;
    }
}
