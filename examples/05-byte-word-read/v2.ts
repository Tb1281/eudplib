/**
 * Scenario 05: Byte and Word Read — Modern eudkit (v2).
 *
 * Ideal Intent:
 * Perform sub-dword memory reads at fixed and dynamic cursor addresses:
 * 1. Low byte (subp=0) and low word (subp=0) from a fixed score table dword (`0x58A364 + 0x40`).
 * 2. Unit type 16-bit word from the CUnit structure header (`0x59CCA8 + 0x64`).
 * 3. Single byte at subp=2 from a dynamic game-time cursor EPD.
 * 4. Pack the retrieved fields into a composite status dword with bit shifts,
 *    incrementing status on a sentinel byte value.
 */

import {
    EPD,
    bread_epd,
    session,
    wread_epd,
} from "eudkit";

export const FIXED_EPD = EPD((0x58A364 + 64));
export const UNIT_TYPE_EPD = EPD((0x59CCA8 + 100));
export let low_byte = session.var(0, "low_byte");
export let low_word = session.var(0, "low_word");
export let unit_type = session.var(0, "unit_type");
export let cursor_epd = session.var(EPD(0x59CCA8), "cursor_epd");
export let cursor_byte = session.var(0, "cursor_byte");
export let status = session.var(0, "status");

export function main(): void {
    low_byte = bread_epd(FIXED_EPD, 0);
    low_word = wread_epd(FIXED_EPD, 0);
    unit_type = wread_epd(UNIT_TYPE_EPD, 0);
    cursor_byte = bread_epd(cursor_epd, 2);
    status = ((low_byte + (low_word << 8)) + (unit_type << 16));
    if ((cursor_byte == 255)) {
        status += 1;
    }
}
