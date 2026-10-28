/**
 * Scenario 17: Unit Snapshot and Restore — Modern eudkit (v2).
 *
 * Ideal Intent:
 * Capture and restore a complete 84-dword (336-byte) CUnit structure:
 * 1. Snapshot request (`snapshot_flag == 1`):
 *    - Copy 84 dwords from CUnit slot 0 (`0x59CCA8`) into a snapshot buffer.
 *    - Verify unit type integrity via `wread_epd`.
 *    - Set `snapshot_valid = 1`, increment `snapshots_done`, clear `snapshot_flag`.
 * 2. Restore request (`restore_flag == 1`):
 *    - If a valid snapshot exists (`snapshot_valid == 1`), copy the 84 dwords back to slot 0.
 *    - Increment `restores_done`, clear `restore_flag`.
 */

import {
    Array,
    EPD,
    dwread_epd,
    dwwrite_epd,
    session,
    wread_epd,
} from "eudkit";

export const CUNIT_TABLE = 0x59CCA8;
export const UNIT_EPD = EPD(CUNIT_TABLE);
export const SNAPSHOT_DWORDS = 84;
export const SNAPSHOT_BUF_EPD = EPD(0x6FE000);
export const UNIT_TYPE_OFFSET = ((100 / 4) | 0);
export let snapshot_flag = session.var(0, "snapshot_flag");
export let restore_flag = session.var(0, "restore_flag");
export let snapshot_valid = session.var(0, "snapshot_valid");
export let snapshots_done = session.var(0, "snapshots_done");
export let restores_done = session.var(0, "restores_done");
export let snapshot_type = session.var(0, "snapshot_type");
export let i = session.var(0, "i");
export const snapshot_buf = new Array(SNAPSHOT_DWORDS);

export function main(): void {
    if ((snapshot_flag == 1)) {
        i = 0;
        while ((i < SNAPSHOT_DWORDS)) {
            let _tmp2 = dwread_epd((UNIT_EPD + i));
            let _tmp1 = _tmp2;
            dwwrite_epd((SNAPSHOT_BUF_EPD + i), _tmp1);
            i += 1;
        }
        snapshot_type = wread_epd((SNAPSHOT_BUF_EPD + UNIT_TYPE_OFFSET), 0);
        snapshot_valid = 1;
        snapshots_done += 1;
        snapshot_flag = 0;
    }
    if ((restore_flag == 1)) {
        if ((snapshot_valid == 1)) {
            i = 0;
            while ((i < SNAPSHOT_DWORDS)) {
                let _tmp4 = dwread_epd((SNAPSHOT_BUF_EPD + i));
                let _tmp3 = _tmp4;
                dwwrite_epd((UNIT_EPD + i), _tmp3);
                i += 1;
            }
            restores_done += 1;
        }
        restore_flag = 0;
    }
}
