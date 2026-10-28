/**
 * Scenario 22: Unit Type Renaming (TBL Patching) — Modern eudkit (v2).
 *
 * Ideal Intent:
 * Rename 8 core combat units at game start via StarCraft's TBL string table:
 * - Terran Marine (TBL id 1) -> "Space Marine"
 * - Terran Ghost (TBL id 2) -> "Shadow Operative"
 * - Terran Firebat (TBL id 33) -> "Pyro Specialist"
 * - Terran Medic (TBL id 35) -> "Field Surgeon"
 * - Zerg Zergling (TBL id 38) -> "Voidling"
 * - Zerg Hydralisk (TBL id 39) -> "Spine Beast"
 * - Protoss Zealot (TBL id 66) -> "Blade Adept"
 * - Protoss Dragoon (TBL id 67) -> "War Strider"
 * Pack strings into 4-byte little-endian dwords and emit writes once on map initialization.
 */

import {
    EPD,
    dwwrite_epd,
    epdread_epd,
    once,
    session,
    wread_epd,
} from "eudkit";

export const ROSTER_RENAMES = [[1, "Space Marine"], [2, "Shadow Operative"], [33, "Pyro Specialist"], [35, "Field Surgeon"], [38, "Voidling"], [39, "Spine Beast"], [66, "Blade Adept"], [67, "War Strider"]];
export const PACKED_RENAMES = [];
export let tbl_epd = session.var(0, "tbl_epd");
export let str_offset = session.var(0, "str_offset");
export let dst_epd = session.var(0, "dst_epd");
export let renamed = session.var(0, "renamed");

export function main(): void {
    once(() => {
        tbl_epd = epdread_epd(EPD(0x6D5A30));
        for (let i = 0; i < 0; i++) {
            let entry = PACKED_RENAMES[i];
            str_offset = wread_epd((tbl_epd + ((entry[0] / 2) | 0)), (entry[0] % 2));
            dst_epd = (tbl_epd + ((str_offset / 4) | 0));
            for (let j = 0; j < entry[1].length; j++) {
                dwwrite_epd((dst_epd + j), entry[1][j]);
            }
        }
        renamed = 1;
    });
}
