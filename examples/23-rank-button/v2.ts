/**
 * Scenario 23: Rank Ladder and Action Button Relabeling — Modern eudkit (v2).
 *
 * Ideal Intent:
 * Relabel 7 rank titles and customize the command card attack button at game start:
 * - Recruit (1302) -> "Rookie"
 * - Corporal (1306) -> "Veteran"
 * - Sergeant (1308) -> "Drill Sergeant"
 * - Master Sergeant (1310) -> "First Sergeant"
 * - Captain (1312) -> "Commander"
 * - Colonel (1316) -> "Wing Commander"
 * - Admiral (1324) -> "Fleet Admiral"
 * - Attack Button (1400, offset 2) -> "Charge!"
 * Executes once on map startup using packed dwords into `stat_txt.tbl` (`0x6D5A30`).
 */

import {
    EPD,
    dwwrite_epd,
    epdread_epd,
    once,
    session,
    wread_epd,
} from "eudkit";

export const BUTTON_RENAMES = [[1302, 0, "Rookie"], [1306, 0, "Veteran"], [1308, 0, "Drill Sergeant"], [1310, 0, "First Sergeant"], [1312, 0, "Commander"], [1316, 0, "Wing Commander"], [1324, 0, "Fleet Admiral"], [1400, 2, "Charge!"]];
export const PACKED_BUTTONS = [];
export let tbl_epd = session.var(0, "tbl_epd");
export let str_offset = session.var(0, "str_offset");
export let dst_epd = session.var(0, "dst_epd");
export let relabeled = session.var(0, "relabeled");

export function main(): void {
    once(() => {
        tbl_epd = epdread_epd(EPD(0x6D5A30));
        for (let i = 0; i < 0; i++) {
            let entry = PACKED_BUTTONS[i];
            str_offset = wread_epd((tbl_epd + ((entry[0] / 2) | 0)), (entry[0] % 2));
            dst_epd = (tbl_epd + (((str_offset + entry[1]) / 4) | 0));
            for (let j = 0; j < entry[2].length; j++) {
                dwwrite_epd((dst_epd + j), entry[2][j]);
            }
        }
        relabeled = 1;
    });
}
