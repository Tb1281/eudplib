/**
 * Scenario 25: Periodic Income and Milestone Economy — Modern eudkit (v2).
 *
 * Ideal Intent:
 * Every 150 trigger loops (approx 6.25 seconds on Fastest speed):
 * 1. Pay 50 Ore and 20 Gas to each active player (Players 1 to 4).
 * 2. Every 4th payout is a milestone payout, awarding an extra 100 Ore to each player.
 * 3. Track total payouts, milestones, and lifetime resource disbursements.
 * Uses declarative reactive timing (`with every(150):`).
 */

import {
    Add,
    Gas,
    Ore,
    SetResources,
    every,
    session,
} from "eudkit";

export const INCOME_PERIOD = 150;
export const ORE_PER_PAYOUT = 50;
export const GAS_PER_PAYOUT = 20;
export const MILESTONE_EVERY = 4;
export const MILESTONE_ORE = 100;
export const PLAYERS_COUNT = 4;
export let payouts = session.var(0, "payouts");
export let milestone_counter = session.var(0, "milestone_counter");
export let milestones = session.var(0, "milestones");
export let total_ore_paid = session.var(0, "total_ore_paid");
export let total_gas_paid = session.var(0, "total_gas_paid");
export let is_milestone = session.var(0, "is_milestone");

export function main(): void {
    every(INCOME_PERIOD, () => {
        payouts += 1;
        milestone_counter += 1;
        is_milestone = 0;
        if ((milestone_counter >= MILESTONE_EVERY)) {
            milestone_counter = 0;
            milestones += 1;
            is_milestone = 1;
        }
        for (let p = 0; p < PLAYERS_COUNT; p++) {
            SetResources(p, Add, ORE_PER_PAYOUT, Ore);
            SetResources(p, Add, GAS_PER_PAYOUT, Gas);
            if ((is_milestone == 1)) {
                SetResources(p, Add, MILESTONE_ORE, Ore);
            }
        }
        total_ore_paid += (ORE_PER_PAYOUT * PLAYERS_COUNT);
        total_gas_paid += (GAS_PER_PAYOUT * PLAYERS_COUNT);
        if ((is_milestone == 1)) {
            total_ore_paid += (MILESTONE_ORE * PLAYERS_COUNT);
        }
    });
}
