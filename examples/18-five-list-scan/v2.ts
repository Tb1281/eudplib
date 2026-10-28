/**
 * Scenario 18: Engine Unit List Census (Hidden Units) — Modern eudkit (v2).
 *
 * Ideal Intent:
 * Census units across multiple engine lists (live units, hidden units, scanner sweeps):
 * 1. Phase 1: Walk the active unit linked list (`0x628430`), increment `live_count`,
 *    and stamp each slot index with a generation token (`mark[slot] = mark_gen`).
 * 2. Phase 2: Walk all 1700 CUnit slots. Valid slots (`unitType != 228`) whose generation mark
 *    does not match `mark_gen` are "hidden" units.
 * 3. Count total hidden units and hidden Terran Marines.
 * 4. Count Scanner Sweep units (`unitType == 33`).
 * 5. Emit milestone death flag if any hidden unit is discovered.
 */

import {
    Array,
    EPD,
    Player1,
    SetDeaths,
    SetTo,
    dwread_epd,
    dwwrite_epd,
    epdread_epd,
    session,
    wread_epd,
} from "eudkit";

export const CUNIT_BASE_EPD = EPD(0x59CCA8);
export const STRIDE = 84;
export const SCANNER_SWEEP = 33;
export const MARINE = 0;
export const NONE_TYPE = 228;
export const MARK_BUF_EPD = EPD(0x6FE000);
export let live_count = session.var(0, "live_count");
export let hidden_count = session.var(0, "hidden_count");
export let hidden_marines = session.var(0, "hidden_marines");
export let scanner_count = session.var(0, "scanner_count");
export let mark_gen = session.var(0, "mark_gen");
export let cur_epd = session.var(0, "cur_epd");
export let slot = session.var(0, "slot");
export let slot_idx = session.var(0, "slot_idx");
export let slot_epd = session.var(0, "slot_epd");
export let unit_type = session.var(0, "unit_type");
export const mark = new Array(1700);

export function main(): void {
    live_count = 0;
    hidden_count = 0;
    hidden_marines = 0;
    scanner_count = 0;
    mark_gen += 1;
    cur_epd = epdread_epd(EPD(0x628430));
    while ((cur_epd != EPD(0))) {
        live_count += 1;
        slot = (((cur_epd - CUNIT_BASE_EPD) / STRIDE) | 0);
        dwwrite_epd(MARK_BUF_EPD + slot, mark_gen);
        cur_epd = epdread_epd((cur_epd + 1));
    }
    slot_idx = 0;
    while ((slot_idx < 1700)) {
        slot_epd = (CUNIT_BASE_EPD + (slot_idx * STRIDE));
        unit_type = wread_epd((slot_epd + 25), 0);
        if ((unit_type != NONE_TYPE)) {
            if ((unit_type == SCANNER_SWEEP)) {
                scanner_count += 1;
            }
            let _tmp1 = dwread_epd(MARK_BUF_EPD + slot_idx);
            if ((_tmp1 != mark_gen)) {
                hidden_count += 1;
                if ((unit_type == MARINE)) {
                    hidden_marines += 1;
                }
            }
        }
        slot_idx += 1;
    }
    if ((hidden_count >= 1)) {
        SetDeaths(Player1, SetTo, 1, 205);
    }
}
