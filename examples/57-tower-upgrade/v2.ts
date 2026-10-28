/**
 *
 */

import {
    Accumulate,
    AtLeast,
    Bring,
    Clear,
    KillUnitAt,
    MinimapPing,
    Ore,
    P1,
    P8,
    Set,
    SetResources,
    SetSwitch,
    Subtract,
    Switch,
    dwread,
    session,
} from "eudkit";

export const P1 = 0;
export const P8 = 7;
export const ZERGLING = 37;
export const HYDRALISK = 38;
export const LOC_TOWER = 3;
export const MAX_LEVEL = 3;
export const BASE_COOLDOWN = 40;
export const CD_PER_LEVEL = 5;
export const UPGRADE_COSTS = [150, 250, 400];
export const SW_UPGRADE = 7;
export const DEATHS_ZERGLING = ((0x58A364 + (4 * P8)) + (48 * ZERGLING));
export const DEATHS_HYDRALISK = ((0x58A364 + (4 * P8)) + (48 * HYDRALISK));
export let tower_level = session.var(0, "tower_level");
export let cooldown = session.var(0, "cooldown");
export let fired = session.var(0, "fired");
export let shots_fired = session.var(0, "shots_fired");
export let tower_kills = session.var(0, "tower_kills");
export let upgrades = session.var(0, "upgrades");
export let failed_buys = session.var(0, "failed_buys");
export let ore_spent = session.var(0, "ore_spent");
export let prev_zergling = session.var(0, "prev_zergling");
export let prev_hydralisk = session.var(0, "prev_hydralisk");

export function main(): void {
    let cur_z = dwread(DEATHS_ZERGLING);
    let cur_h = dwread(DEATHS_HYDRALISK);
    if ((cur_z > prev_zergling)) {
        tower_kills += (cur_z - prev_zergling);
    }
    if ((cur_h > prev_hydralisk)) {
        tower_kills += (cur_h - prev_hydralisk);
    }
    prev_zergling = cur_z;
    prev_hydralisk = cur_h;
    if (Switch(SW_UPGRADE, Set)) {
        SetSwitch(SW_UPGRADE, Clear);
        if ((tower_level == 0)) {
            if (Accumulate(P1, AtLeast, UPGRADE_COSTS[0], Ore)) {
                SetResources(P1, Subtract, UPGRADE_COSTS[0], Ore);
                tower_level += 1;
                upgrades += 1;
                ore_spent += UPGRADE_COSTS[0];
                MinimapPing(LOC_TOWER);
            } else {
                failed_buys += 1;
            }
        } else if ((tower_level == 1)) {
            if (Accumulate(P1, AtLeast, UPGRADE_COSTS[1], Ore)) {
                SetResources(P1, Subtract, UPGRADE_COSTS[1], Ore);
                tower_level += 1;
                upgrades += 1;
                ore_spent += UPGRADE_COSTS[1];
                MinimapPing(LOC_TOWER);
            } else {
                failed_buys += 1;
            }
        } else if ((tower_level == 2)) {
            if (Accumulate(P1, AtLeast, UPGRADE_COSTS[2], Ore)) {
                SetResources(P1, Subtract, UPGRADE_COSTS[2], Ore);
                tower_level += 1;
                upgrades += 1;
                ore_spent += UPGRADE_COSTS[2];
                MinimapPing(LOC_TOWER);
            } else {
                failed_buys += 1;
            }
        } else {
            failed_buys += 1;
        }
    }
    if ((cooldown >= 1)) {
        cooldown -= 1;
    }
    let has_target = 0;
    if (Bring(P8, AtLeast, 1, ZERGLING, LOC_TOWER)) {
        has_target = 1;
    }
    if (Bring(P8, AtLeast, 1, HYDRALISK, LOC_TOWER)) {
        has_target = 1;
    }
    if (((cooldown == 0) && (has_target == 1))) {
        fired = 0;
        while ((fired <= tower_level)) {
            if (Bring(P8, AtLeast, 1, ZERGLING, LOC_TOWER)) {
                KillUnitAt(1, ZERGLING, LOC_TOWER, P8);
                fired += 1;
                shots_fired += 1;
            } else if (Bring(P8, AtLeast, 1, HYDRALISK, LOC_TOWER)) {
                KillUnitAt(1, HYDRALISK, LOC_TOWER, P8);
                fired += 1;
                shots_fired += 1;
            } else {
                fired = 99;
            }
        }
        cooldown = (BASE_COOLDOWN - (tower_level * CD_PER_LEVEL));
    }
}
