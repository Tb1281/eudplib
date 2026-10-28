/**
 * Scenario 03: Block Copy (CUnit Snapshot) — Modern eudkit (v2).
 *
 * Ideal Intent:
 * Whenever a snapshot request is triggered (`request_flag == 1`), copy one unit's
 * complete CUnit record (336 bytes = 84 dwords) from dynamic `source_epd` into a
 * designated snapshot buffer.
 * Bookkeeping:
 * - Set reentrancy guard (`copy_busy`)
 * - Copy all 84 dwords
 * - Read back and verify the unitType field (offset +0x64 = +25 dwords)
 * - Record last source EPD, increment snapshot completion counter, and release flags.
 */

import {
    Array,
    EPD,
    dwread_epd,
    dwwrite_epd,
    session,
} from "eudkit";

export const SNAPSHOT_DWORDS = 84;
export const SNAPSHOT_BUF_EPD = EPD(0x6FE000);
export const UNIT_TYPE_OFFSET = ((100 / 4) | 0);
export let request_flag = session.var(0, "request_flag");
export let source_epd = session.var(EPD(0x59CCA8), "source_epd");
export let snapshots_done = session.var(0, "snapshots_done");
export let last_source_epd = session.var(0, "last_source_epd");
export let copy_busy = session.var(0, "copy_busy");
export let snapshot_type = session.var(0, "snapshot_type");
export let i = session.var(0, "i");
export const snapshot_buf = new Array(SNAPSHOT_DWORDS);

export function main(): void {
    if ((request_flag == 1)) {
        if ((copy_busy == 0)) {
            copy_busy = 1;
            i = 0;
            while ((i < SNAPSHOT_DWORDS)) {
                let _tmp2 = dwread_epd((source_epd + i));
                let _tmp1 = _tmp2;
                dwwrite_epd((SNAPSHOT_BUF_EPD + i), _tmp1);
                i += 1;
            }
            snapshot_type = dwread_epd((SNAPSHOT_BUF_EPD + UNIT_TYPE_OFFSET));
            last_source_epd = source_epd;
            snapshots_done += 1;
            request_flag = 0;
            copy_busy = 0;
        }
    }
}
