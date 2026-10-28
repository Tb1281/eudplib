/**
 *
 */

import {
    Add,
    Gas,
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
export const LOC_BASE = 1;
export const ZERGLING = 37;
export const HYDRALISK = 38;
export const ULTRALISK = 39;
export const XP_ZERGLING = 5;
export const XP_HYDRALISK = 15;
export const XP_ULTRALISK = 40;
export const MAX_LEVEL = 10;
export const LEVEL_ORE = 150;
export const LEVEL_GAS = 25;
export const DEATHS_ZERGLING = ((0x58A364 + (4 * P8)) + (48 * ZERGLING));
export const DEATHS_HYDRALISK = ((0x58A364 + (4 * P8)) + (48 * HYDRALISK));
export const DEATHS_ULTRALISK = ((0x58A364 + (4 * P8)) + (48 * ULTRALISK));
export let level = session.var(1, "level");
export let xp_in_level = session.var(0, "xp_in_level");
export let total_xp = session.var(0, "total_xp");
export let level_ups = session.var(0, "level_ups");
export let kills_zergling = session.var(0, "kills_zergling");
export let kills_hydralisk = session.var(0, "kills_hydralisk");
export let kills_ultralisk = session.var(0, "kills_ultralisk");
export let prev_z = session.var(0, "prev_z");
export let prev_h = session.var(0, "prev_h");
export let prev_u = session.var(0, "prev_u");

function need_for(l: number): number {
    return (100 + (20 * (l - 1)));
}

export function main(): void {
    let cur_z = dwread(DEATHS_ZERGLING);
    let cur_h = dwread(DEATHS_HYDRALISK);
    let cur_u = dwread(DEATHS_ULTRALISK);
    let gained = 0;
    if ((cur_z > prev_z)) {
        let dz = (cur_z - prev_z);
        gained += (dz * XP_ZERGLING);
        kills_zergling += dz;
    }
    if ((cur_h > prev_h)) {
        let dh = (cur_h - prev_h);
        gained += (dh * XP_HYDRALISK);
        kills_hydralisk += dh;
    }
    if ((cur_u > prev_u)) {
        let du = (cur_u - prev_u);
        gained += (du * XP_ULTRALISK);
        kills_ultralisk += du;
    }
    prev_z = cur_z;
    prev_h = cur_h;
    prev_u = cur_u;
    if ((gained >= 1)) {
        xp_in_level += gained;
        total_xp += gained;
    }
    for (let l = 1; l < MAX_LEVEL; l++) {
        let req = (100 + (20 * (l - 1)));
        if ((level == l)) {
            if ((xp_in_level >= req)) {
                xp_in_level -= req;
                level += 1;
                level_ups += 1;
                SetResources(P1, Add, LEVEL_ORE, Ore);
                SetResources(P1, Add, LEVEL_GAS, Gas);
                MinimapPing(LOC_BASE);
            }
        }
    }
}
