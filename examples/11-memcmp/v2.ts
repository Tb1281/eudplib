/**
 * Scenario 11: Memory Compare (Buffer Equality) — Modern eudkit (v2).
 *
 * Ideal Intent:
 * Compare two 4-dword (16-byte) configuration buffers (primary and shadow) for equality:
 * 1. Check first half (dwords 0 and 1) and second half (dwords 2 and 3).
 * 2. Raise `resync_flag = 1` if ANY dword differs between buffers.
 * 3. Identify which half diverged for diagnostics (`diff_half = 1` for first half, `2` for second half, `0` if equal).
 */

import {
    EPD,
    dwread_epd,
    session,
} from "eudkit";

export const BUF_A_EPD = EPD((0x58A364 + 2048));
export const BUF_B_EPD = EPD((0x58A364 + 2112));
export let resync_flag = session.var(0, "resync_flag");
export let diff_half = session.var(0, "diff_half");
export let diff_first = session.var(0, "diff_first");
export let diff_second = session.var(0, "diff_second");

export function main(): void {
    diff_first = 0;
    diff_second = 0;
    let _tmp1 = dwread_epd((BUF_A_EPD + 0));
    let _tmp2 = dwread_epd((BUF_B_EPD + 0));
    if ((_tmp1 != _tmp2)) {
        diff_first = 1;
    }
    let _tmp3 = dwread_epd((BUF_A_EPD + 1));
    let _tmp4 = dwread_epd((BUF_B_EPD + 1));
    if ((_tmp3 != _tmp4)) {
        diff_first = 1;
    }
    let _tmp5 = dwread_epd((BUF_A_EPD + 2));
    let _tmp6 = dwread_epd((BUF_B_EPD + 2));
    if ((_tmp5 != _tmp6)) {
        diff_second = 1;
    }
    let _tmp7 = dwread_epd((BUF_A_EPD + 3));
    let _tmp8 = dwread_epd((BUF_B_EPD + 3));
    if ((_tmp7 != _tmp8)) {
        diff_second = 1;
    }
    if (((diff_first != 0) || (diff_second != 0))) {
        resync_flag = 1;
    } else {
        resync_flag = 0;
    }
    if ((diff_first != 0)) {
        diff_half = 1;
    } else if ((diff_second != 0)) {
        diff_half = 2;
    } else {
        diff_half = 0;
    }
}
