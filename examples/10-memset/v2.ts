/**
 * Scenario 10: Memset (Buffer Clear) — Modern eudkit (v2).
 *
 * Ideal Intent:
 * Clear a 64-dword scratch region on demand:
 * 1. Detect request flag (`clear_flag != 0`).
 * 2. Iterate through 64 dwords, setting each element to 0 via direct memory write.
 * 3. Reset request flag to 0 and increment completion counter (`clears_done += 1`).
 */

import {
    EPD,
    dwwrite_epd,
    session,
} from "eudkit";

export const SCRATCH_EPD = EPD(0x6FE000);
export let clear_flag = session.var(0, "clear_flag");
export let idx = session.var(0, "idx");
export let clears_done = session.var(0, "clears_done");

export function main(): void {
    if ((clear_flag != 0)) {
        idx = 0;
        while ((idx < 64)) {
            dwwrite_epd((SCRATCH_EPD + idx), 0);
            idx += 1;
        }
        clear_flag = 0;
        clears_done += 1;
    }
}
