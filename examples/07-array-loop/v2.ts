/**
 * Scenario 07: Array Loop (Sum and In-Place Double) — Modern eudkit (v2).
 *
 * Ideal Intent:
 * Process a 16-dword array in two sequential loops:
 * 1. Pass 1: Sum all 16 array elements into a running total variable.
 * 2. Pass 2: In-place double every element (`arr[idx] = arr[idx] * 2`).
 * 3. Compute a final 32-bit checksum (`total + (total >> 16)`).
 */

import {
    Array,
    EPD,
    dwread_epd,
    dwwrite_epd,
    session,
} from "eudkit";

export const ARR_EPD = EPD(0x6FE000);
export const arr = new Array(/* expr */);
export let total = session.var(0, "total");
export let idx = session.var(0, "idx");
export let checksum = session.var(0, "checksum");

export function main(): void {
    total = 0;
    idx = 0;
    while ((idx < 16)) {
        let _tmp1 = dwread_epd(ARR_EPD + idx);
        total += _tmp1;
        idx += 1;
    }
    idx = 0;
    while ((idx < 16)) {
        let _tmp2 = dwread_epd(ARR_EPD + idx);
        dwwrite_epd(ARR_EPD + idx, (_tmp2 * 2));
        idx += 1;
    }
    checksum = (total + (total >> 16));
}
