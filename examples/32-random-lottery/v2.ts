/**
 * Scenario 32: Random Lottery and Prize Tier Distribution — Modern eudkit (v2).
 *
 * Ideal Intent:
 * Every 200 trigger loops, each of 4 active players (Players 1 to 4) draws a lottery roll (0..99):
 * 1. Prize tiers:
 *    - Roll < 5: 500 Ore (Jackpot tier, increments jackpots counter)
 *    - Roll < 25: 100 Ore (Big tier)
 *    - Roll < 60: 25 Ore (Small tier)
 *    - Roll >= 60: No prize
 * 2. Track total draws (`draws`), total ore disbursed (`total_paid`), and lowest roll seen (`best_roll`).
 * Modern v2 uses:
 * - Declarative timing (`with every(200):`).
 * - Inlined fast LCG pseudo-random generator eliminating external EUDFunc trampolines.
 * - Direct constant initialization (`best_roll = session.var(100)`) eliminating legacy `seen_any` guard flags.
 */

import {
    Add,
    Ore,
    SetResources,
    every,
    session,
} from "eudkit";

export const LOTTERY_PERIOD = 200;
export const JACKPOT_ROLL = 5;
export const BIG_ROLL = 25;
export const SMALL_ROLL = 60;
export const JACKPOT_ORE = 500;
export const BIG_ORE = 100;
export const SMALL_ORE = 25;
export const PLAYERS_COUNT = 4;
export let seed = session.var(0x12D687, "seed");
export let draws = session.var(0, "draws");
export let jackpots = session.var(0, "jackpots");
export let total_paid = session.var(0, "total_paid");
export let best_roll = session.var(100, "best_roll");
export let roll = session.var(0, "roll");

export function main(): void {
    every(LOTTERY_PERIOD, () => {
        for (let p = 0; p < PLAYERS_COUNT; p++) {
            seed = ((seed * 0x19660D) + 0x3C6EF35F);
            roll = (seed % 100);
            draws += 1;
            if ((roll < JACKPOT_ROLL)) {
                SetResources(p, Add, JACKPOT_ORE, Ore);
                total_paid += JACKPOT_ORE;
                jackpots += 1;
            } else if ((roll < BIG_ROLL)) {
                SetResources(p, Add, BIG_ORE, Ore);
                total_paid += BIG_ORE;
            } else if ((roll < SMALL_ROLL)) {
                SetResources(p, Add, SMALL_ORE, Ore);
                total_paid += SMALL_ORE;
            }
            if ((roll < best_roll)) {
                best_roll = roll;
            }
        }
    });
}
