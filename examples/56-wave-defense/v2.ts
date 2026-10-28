/**
 *
 */

import {
    Add,
    AtLeast,
    Bring,
    CreateUnit,
    Gas,
    KillUnitAt,
    MinimapPing,
    Ore,
    P1,
    P8,
    SetResources,
    dwread,
    session,
} from "eudkit";

export const P1 = 0;
export const P8 = 7;
export const ZERGLING = 37;
export const HYDRALISK = 38;
export const LOC_SPAWN = 1;
export const LOC_BASE = 2;
export const MAX_WAVE = 8;
export const INTERMISSION = 120;
export const SPAWN_PERIOD = 6;
export const START_LIVES = 10;
export const CLEAR_ORE = 150;
export const BONUS_GAS = 75;
export const BONUS_EVERY = 3;
export const DEATHS_ZERGLING = ((0x58A364 + (4 * P8)) + (48 * ZERGLING));
export const DEATHS_HYDRALISK = ((0x58A364 + (4 * P8)) + (48 * HYDRALISK));
export let state = session.var(0, "state");
export let wave = session.var(1, "wave");
export let inter_timer = session.var(0, "inter_timer");
export let spawn_timer = session.var(0, "spawn_timer");
export let spawn_left = session.var(0, "spawn_left");
export let alive = session.var(0, "alive");
export let spawn_alt = session.var(0, "spawn_alt");
export let lives = session.var(START_LIVES, "lives");
export let victory = session.var(0, "victory");
export let defeat = session.var(0, "defeat");
export let waves_cleared = session.var(0, "waves_cleared");
export let bonus_counter = session.var(0, "bonus_counter");
export let total_spawned = session.var(0, "total_spawned");
export let leaks = session.var(0, "leaks");
export let prev_zergling = session.var(0, "prev_zergling");
export let prev_hydralisk = session.var(0, "prev_hydralisk");

export function main(): void {
    if (((victory == 0) && (defeat == 0))) {
        let cur_z = dwread(DEATHS_ZERGLING);
        let cur_h = dwread(DEATHS_HYDRALISK);
        if ((cur_z > prev_zergling)) {
            alive -= (cur_z - prev_zergling);
        }
        if ((cur_h > prev_hydralisk)) {
            alive -= (cur_h - prev_hydralisk);
        }
        prev_zergling = cur_z;
        prev_hydralisk = cur_h;
        if ((state == 1)) {
            if (Bring(P8, AtLeast, 1, ZERGLING, LOC_BASE)) {
                KillUnitAt(1, ZERGLING, LOC_BASE, P8);
                lives -= 1;
                leaks += 1;
            }
            if (Bring(P8, AtLeast, 1, HYDRALISK, LOC_BASE)) {
                KillUnitAt(1, HYDRALISK, LOC_BASE, P8);
                lives -= 1;
                leaks += 1;
            }
            if ((lives == 0)) {
                defeat = 1;
            }
        }
        if ((state == 0)) {
            inter_timer += 1;
            if ((inter_timer >= INTERMISSION)) {
                inter_timer = 0;
                spawn_left = (6 + (wave * 3));
                spawn_timer = 0;
                state = 1;
                MinimapPing(LOC_SPAWN);
            }
        }
        if ((state == 1)) {
            if ((spawn_left >= 1)) {
                spawn_timer += 1;
                if ((spawn_timer >= SPAWN_PERIOD)) {
                    spawn_timer = 0;
                    spawn_left -= 1;
                    alive += 1;
                    total_spawned += 1;
                    if ((spawn_alt == 0)) {
                        CreateUnit(1, ZERGLING, LOC_SPAWN, P8);
                        spawn_alt = 1;
                    } else {
                        CreateUnit(1, HYDRALISK, LOC_SPAWN, P8);
                        spawn_alt = 0;
                    }
                }
            }
            if (((spawn_left == 0) && (alive == 0))) {
                SetResources(P1, Add, CLEAR_ORE, Ore);
                waves_cleared += 1;
                bonus_counter += 1;
                if ((bonus_counter >= BONUS_EVERY)) {
                    bonus_counter = 0;
                    SetResources(P1, Add, BONUS_GAS, Gas);
                }
                if ((wave >= MAX_WAVE)) {
                    victory = 1;
                } else {
                    wave += 1;
                    state = 0;
                    inter_timer = 0;
                }
            }
        }
    }
}
