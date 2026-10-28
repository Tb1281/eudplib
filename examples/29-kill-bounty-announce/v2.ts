/**
 * Scenario 29: Kill Bounty and Broadcast Announcement — Modern eudkit (v2).
 *
 * Ideal Intent:
 * Monitors Player 1 Marine deaths counter:
 * 1. When counter increases, compute delta `new_kills = cur_kills - prev_kills`.
 * 2. Compute bounty: 15 ore per kill.
 * 3. If `new_kills >= 5` (jackpot), add a flat 50-ore bonus and increment `jackpots`.
 * 4. Pay ore by updating Player 1's ore dword at `0x57F0F0`.
 * 5. Formats and displays an in-game broadcast message using modern StringBuffer:
 *    `"Player 1 scored %d kills! Bounty: %d ore"`
 * 6. Track cumulative statistics: `total_kills_paid`, `total_bounty`, `announcements`.
 */

import {
    Add,
    EPD,
    SetDeaths,
    dwread_epd,
    session,
} from "eudkit";

export const MARINE_DEATHS_P1 = 0x58A364;
export const ORE_ADDR_P1 = 0x57F0F0;
export const BOUNTY_PER_KILL = 15;
export const JACKPOT_MIN_KILLS = 5;
export const JACKPOT_BONUS = 50;
export let announce_buf = session.string(128);
export let prev_kills = session.var(0, "prev_kills");
export let cur_kills = session.var(0, "cur_kills");
export let new_kills = session.var(0, "new_kills");
export let bounty = session.var(0, "bounty");
export let total_kills_paid = session.var(0, "total_kills_paid");
export let total_bounty = session.var(0, "total_bounty");
export let jackpots = session.var(0, "jackpots");
export let announcements = session.var(0, "announcements");

export function main(): void {
    cur_kills = dwread_epd(EPD(MARINE_DEATHS_P1));
    if ((cur_kills > prev_kills)) {
        new_kills = (cur_kills - prev_kills);
        bounty = (new_kills * BOUNTY_PER_KILL);
        if ((new_kills >= JACKPOT_MIN_KILLS)) {
            bounty += JACKPOT_BONUS;
            jackpots += 1;
        }
        SetDeaths(EPD(ORE_ADDR_P1), Add, bounty, 0);
        total_kills_paid += new_kills;
        total_bounty += bounty;
        announce_buf.printf("Player 1 scored %d kills! Bounty: %d ore", new_kills, bounty);
        announce_buf.display();
        announcements += 1;
    }
    prev_kills = cur_kills;
}
