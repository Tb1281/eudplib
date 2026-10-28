/**
 * Scenario 09: Struct-Array Element Field Access — Modern eudkit (v2).
 *
 * Ideal Intent:
 * Manage a table of 8 structured unit records (84 dwords per record, matching CUnit stride):
 * 1. Pass 1: Find the first free slot (`hp == 0`), respawn it (`hp = 1000`), record 1-based index,
 *    and terminate loop early (`break`).
 * 2. Pass 2: Iterate over live units (`hp != 0`) and decrement energy (`energy - 1`) if `energy >= 1`.
 */

import {
    EPD,
    dwread_epd,
    dwwrite_epd,
    session,
} from "eudkit";

export const STRIDE = 84;
export const DW_HP = 2;
export const DW_ENERGY = 48;
export const REC_BUF_EPD = EPD(0x6FE000);
export let idx = session.var(0, "idx");
export let respawned = session.var(0, "respawned");
export let slot_epd = session.var(0, "slot_epd");
export let hp = session.var(0, "hp");
export let energy = session.var(0, "energy");

export function main(): void {
    idx = 0;
    respawned = 0;
    while ((idx < 8)) {
        slot_epd = (REC_BUF_EPD + (idx * STRIDE));
        hp = dwread_epd((slot_epd + DW_HP));
        if ((hp == 0)) {
            dwwrite_epd((slot_epd + DW_HP), 1000);
            respawned = (idx + 1);
            break;
        }
        idx += 1;
    }
    idx = 0;
    while ((idx < 8)) {
        slot_epd = (REC_BUF_EPD + (idx * STRIDE));
        hp = dwread_epd((slot_epd + DW_HP));
        energy = dwread_epd((slot_epd + DW_ENERGY));
        if ((hp != 0)) {
            if ((energy >= 1)) {
                dwwrite_epd((slot_epd + DW_ENERGY), (energy - 1));
            }
        }
        idx += 1;
    }
}
