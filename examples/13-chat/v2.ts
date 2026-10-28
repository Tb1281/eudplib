/**
 * Scenario 13: In-Game Chat Detection — Modern eudkit (v2).
 *
 * Ideal Intent:
 * Detect and process in-game chat events from StarCraft engine memory (`0x58D900` / `0x58D904`):
 * 1. Detect chat message arrival (`CHAT_MSG_EPD != 0`).
 * 2. Lazily read sender ID (`CHAT_SENDER_EPD`) only when a message is actually present.
 * 3. Record `last_sender`, increment `chat_count`.
 * 4. Illuminate `p1_lamp` if the sender is Player 1 (`sender == 0`), else turn it off.
 */

import {
    EPD,
    dwread_epd,
    session,
} from "eudkit";

export const CHAT_SENDER_EPD = EPD(0x58D900);
export const CHAT_MSG_EPD = EPD(0x58D904);
export let last_sender = session.var(255, "last_sender");
export let chat_count = session.var(0, "chat_count");
export let p1_lamp = session.var(0, "p1_lamp");
export let sender = session.var(0, "sender");

export function main(): void {
    let _tmp1 = dwread_epd(CHAT_MSG_EPD);
    if ((_tmp1 != 0)) {
        sender = dwread_epd(CHAT_SENDER_EPD);
        last_sender = sender;
        chat_count += 1;
        if ((sender == 0)) {
            p1_lamp = 1;
        } else {
            p1_lamp = 0;
        }
    }
}
