/**
 *
 */

import {
    Accumulate,
    AtLeast,
    Clear,
    CreateUnit,
    Gas,
    MinimapPing,
    Ore,
    P1,
    Set,
    SetResources,
    SetSwitch,
    Subtract,
    Switch,
    session,
} from "eudkit";

export const P1 = 0;
export const MARINE = 0;
export const FIREBAT = 10;
export const MEDIC = 34;
export const LOC_BASE = 1;
export const MAX_ATTACK_LEVEL = 3;
export const bought = [session.var(0, "bought_0"), session.var(0, "bought_1"), session.var(0, "bought_2"), session.var(0, "bought_3")];
export let attack_level = session.var(0, "attack_level");
export let ore_spent = session.var(0, "ore_spent");
export let gas_spent = session.var(0, "gas_spent");
export let failures = session.var(0, "failures");
export let last_failed_item = session.var(0, "last_failed_item");

export function main(): void {
    if (Switch(0, Set)) {
        SetSwitch(0, Clear);
        if (Accumulate(P1, AtLeast, 100, Ore)) {
            SetResources(P1, Subtract, 100, Ore);
            ore_spent += 100;
            CreateUnit(4, MARINE, LOC_BASE, P1);
            bought[0] += 1;
        } else {
            failures += 1;
            last_failed_item = 0;
            MinimapPing(LOC_BASE);
        }
    }
    if (Switch(1, Set)) {
        SetSwitch(1, Clear);
        if (Accumulate(P1, AtLeast, 75, Ore)) {
            SetResources(P1, Subtract, 75, Ore);
            ore_spent += 75;
            CreateUnit(2, MEDIC, LOC_BASE, P1);
            bought[1] += 1;
        } else {
            failures += 1;
            last_failed_item = 1;
            MinimapPing(LOC_BASE);
        }
    }
    if (Switch(2, Set)) {
        SetSwitch(2, Clear);
        if (Accumulate(P1, AtLeast, 125, Ore)) {
            SetResources(P1, Subtract, 125, Ore);
            ore_spent += 125;
            CreateUnit(2, FIREBAT, LOC_BASE, P1);
            bought[2] += 1;
        } else {
            failures += 1;
            last_failed_item = 2;
            MinimapPing(LOC_BASE);
        }
    }
    if (Switch(3, Set)) {
        SetSwitch(3, Clear);
        let can_buy_upg = 0;
        if ((attack_level < MAX_ATTACK_LEVEL)) {
            if (Accumulate(P1, AtLeast, 200, Ore)) {
                if (Accumulate(P1, AtLeast, 50, Gas)) {
                    can_buy_upg = 1;
                }
            }
        }
        if ((can_buy_upg == 1)) {
            SetResources(P1, Subtract, 200, Ore);
            SetResources(P1, Subtract, 50, Gas);
            ore_spent += 200;
            gas_spent += 50;
            attack_level += 1;
            bought[3] += 1;
        } else {
            failures += 1;
            last_failed_item = 3;
            MinimapPing(LOC_BASE);
        }
    }
}
