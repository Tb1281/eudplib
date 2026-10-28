/**
 * Scenario 14: Unit Table Scan and Filter/Count — Modern eudkit (v2).
 *
 * Ideal Intent:
 * Perform a full scan over all 1700 CUnit slots every frame:
 * 1. Identify all active live units (`unitType != 228`).
 * 2. Filter for Player 1 units (`owner == 0`).
 * 3. Count Player 1 Terran Marines (`unitType == 0`).
 * 4. Categorize Marine health:
 *    - HP >= 256 -> `p1_veterans`
 *    - HP < 256 -> `p1_wounded`
 * 5. Update scoreboard flags when veteran/wounded milestones are met.
 */

import {
    EPD,
    Player1,
    SetDeaths,
    SetTo,
    bread_epd,
    dwread_epd,
    session,
    wread_epd,
} from "eudkit";

export const CUNIT_BASE_EPD = EPD(0x59CCA8);
export const STRIDE = 84;
export const NONE_TYPE = 228;
export const MARINE = 0;
export let total_live = session.var(0, "total_live");
export let p1_veterans = session.var(0, "p1_veterans");
export let p1_wounded = session.var(0, "p1_wounded");
export let p1_marines = session.var(0, "p1_marines");
export let i = session.var(0, "i");
export let slot_epd = session.var(0, "slot_epd");
export let unit_type = session.var(0, "unit_type");
export let owner = session.var(0, "owner");
export let hp = session.var(0, "hp");

export function main(): void {
    total_live = 0;
    p1_veterans = 0;
    p1_wounded = 0;
    p1_marines = 0;
    i = 0;
    while ((i < 1700)) {
        slot_epd = (CUNIT_BASE_EPD + (i * STRIDE));
        unit_type = wread_epd((slot_epd + 25), 0);
        if ((unit_type != NONE_TYPE)) {
            total_live += 1;
            owner = bread_epd((slot_epd + 19), 0);
            if ((owner == 0)) {
                if ((unit_type == MARINE)) {
                    p1_marines += 1;
                    hp = dwread_epd((slot_epd + 2));
                    if ((hp >= 256)) {
                        p1_veterans += 1;
                    } else {
                        p1_wounded += 1;
                    }
                }
            }
        }
        i += 1;
    }
    if ((p1_veterans >= 10)) {
        SetDeaths(Player1, SetTo, 1, 200);
    }
    if ((p1_wounded >= 1)) {
        SetDeaths(Player1, SetTo, 1, 201);
    }
}
