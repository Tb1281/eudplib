/**
 * Scenario 01: Kill Counter — Modern eudkit (v2).
 *
 * Ideal Intent:
 * An escalating-threshold kill reward system. Every trigger loop, the engine's
 * Marine deaths counter for Player1 is evaluated against an escalating threshold.
 * When reached:
 * - 100 Ore is awarded via typed SetResources
 * - The deaths counter is reset to 0 via SetDeaths
 * - Consumed kills are credited to lifetime stats
 * - The threshold escalates by +15 kills
 * - Every 3rd payout awards an extra wave bonus of 50 Ore and tracks best wave stats.
 */

import {
    Add,
    AtLeast,
    Deaths,
    Ore,
    Player1,
    SetDeaths,
    SetResources,
    SetTo,
    once,
    session,
} from "eudkit";

export let kills_credited = session.var(0, "kills_credited");
export let next_reward = session.var(10, "next_reward");
export let rewards_paid = session.var(0, "rewards_paid");
export let rewards_this_wave = session.var(0, "rewards_this_wave");
export let ore_paid_total = session.var(0, "ore_paid_total");
export let best_wave = session.var(0, "best_wave");

export function main(): void {
    once(() => {
        next_reward = 10;
        rewards_this_wave = 0;
    });
    if (Deaths(Player1, AtLeast, next_reward, "Terran Marine")) {
        SetResources(Player1, Add, 100, Ore);
        ore_paid_total += 100;
        SetDeaths(Player1, SetTo, 0, "Terran Marine");
        kills_credited += next_reward;
        next_reward += 15;
        rewards_paid += 1;
        rewards_this_wave += 1;
        if ((rewards_this_wave >= 3)) {
            SetResources(Player1, Add, 50, Ore);
            ore_paid_total += 50;
            if ((rewards_this_wave > best_wave)) {
                best_wave = rewards_this_wave;
            }
            rewards_this_wave = 0;
        }
    }
}
