/**
 * Scenario 12: Player Nickname Reader — Modern eudkit (v2).
 *
 * Ideal Intent:
 * Read the first 4 bytes of a player's nickname from the player name table (`0x57EEE8`):
 * - Each player record is 36 bytes (9 dwords).
 * - The player nickname string starts at offset +3 bytes into the record.
 * - Byte 0 is at `base_epd, subp 3`.
 * - Bytes 1..3 are at `base_epd + 1, subp 0..2`.
 * - Pack the 4 characters into a composite `name_sum` dword.
 * - Set `name_flag = 1` if the name starts with uppercase 'A' (`0x41`), else `0`.
 */

import {
    EPD,
    bread_epd,
    session,
} from "eudkit";

export const NAME_TABLE_EPD = EPD(0x57EEE8);
export const STRIDE_DWORDS = 9;
export let target_player = session.var(1, "target_player");
export let name_flag = session.var(0, "name_flag");
export let name_sum = session.var(0, "name_sum");
export let base_epd = session.var(0, "base_epd");
export let b0 = session.var(0, "b0");
export let b1 = session.var(0, "b1");
export let b2 = session.var(0, "b2");
export let b3 = session.var(0, "b3");

export function main(): void {
    base_epd = (NAME_TABLE_EPD + (target_player * STRIDE_DWORDS));
    b0 = bread_epd(base_epd, 3);
    b1 = bread_epd((base_epd + 1), 0);
    b2 = bread_epd((base_epd + 1), 1);
    b3 = bread_epd((base_epd + 1), 2);
    name_sum = (((b0 + (b1 << 8)) + (b2 << 16)) + (b3 << 24));
    if ((b0 == 65)) {
        name_flag = 1;
    } else {
        name_flag = 0;
    }
}
