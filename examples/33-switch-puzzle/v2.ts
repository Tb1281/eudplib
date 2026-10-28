/**
 * Scenario 33: Switch Toggle Puzzle and Near-Miss Hints — Modern eudkit (v2).
 *
 * Ideal Intent:
 * A puzzle door guarded by 4 switches (switches 0..3):
 * - Correct combination: switch 0 Set, switch 1 Cleared, switch 2 Set, switch 3 Set.
 * - Each loop, re-evaluate matching switch count (`match_count`).
 * - When match_count == 4 (solved edge):
 *   - Award 300 Ore to Player 1, ping door location (location 7), increment `solves`.
 *   - Latches `solved_latch = 1` until combination breaks (`match_count <= 3`).
 * - When match_count == 3 (near-miss):
 *   - Increment `near_miss_loops`.
 *   - Throttled hint ping: if `hint_cd == 0`, ping door, increment `hints`, reset cooldown to 240.
 */

import {
    Add,
    Cleared,
    MinimapPing,
    Ore,
    Player1,
    Set,
    SetResources,
    Switch,
    session,
} from "eudkit";

export const SW_A = 0;
export const SW_B = 1;
export const SW_C = 2;
export const SW_D = 3;
export const LOC_DOOR = 7;
export const SOLVE_ORE = 300;
export const HINT_COOLDOWN = 240;
export let match_count = session.var(0, "match_count");
export let solved_latch = session.var(0, "solved_latch");
export let solves = session.var(0, "solves");
export let near_miss_loops = session.var(0, "near_miss_loops");
export let hints = session.var(0, "hints");
export let hint_cd = session.var(0, "hint_cd");

export function main(): void {
    if ((hint_cd >= 1)) {
        hint_cd -= 1;
    }
    match_count = 0;
    if (Switch(SW_A, Set)) {
        match_count += 1;
    }
    if (Switch(SW_B, Cleared)) {
        match_count += 1;
    }
    if (Switch(SW_C, Set)) {
        match_count += 1;
    }
    if (Switch(SW_D, Set)) {
        match_count += 1;
    }
    if (((match_count >= 4) && (solved_latch == 0))) {
        SetResources(Player1, Add, SOLVE_ORE, Ore);
        MinimapPing(LOC_DOOR);
        solves += 1;
        solved_latch = 1;
    }
    if (((match_count <= 3) && (solved_latch == 1))) {
        solved_latch = 0;
    }
    if ((match_count == 3)) {
        near_miss_loops += 1;
        if ((hint_cd == 0)) {
            MinimapPing(LOC_DOOR);
            hints += 1;
            hint_cd = HINT_COOLDOWN;
        }
    }
}
