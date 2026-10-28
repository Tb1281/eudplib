/**
 *
 */

import {
    Accumulate,
    AtLeast,
    AtMost,
    Bring,
    CreateUnit,
    MinimapPing,
    Ore,
    P1,
    RemoveUnitAt,
    SetResources,
    Subtract,
    session,
} from "eudkit";

export const P1 = 0;
export const MARINE = 0;
export const FIREBAT = 10;
export const SIEGE_TANK = 5;
export const LOC_PAD = 4;
export const R1_COST = 50;
export const R2_COST = 100;
export const FUSION_COOLDOWN = 30;
export let cooldown = session.var(0, "cooldown");
export let fusions1 = session.var(0, "fusions1");
export let fusions2 = session.var(0, "fusions2");
export let failed_attempts = session.var(0, "failed_attempts");
export let materials_now = session.var(0, "materials_now");
export let prev_materials = session.var(0, "prev_materials");

export function main(): void {
    if ((cooldown >= 1)) {
        cooldown -= 1;
    }
    let has_firebats = 0;
    let has_marines = 0;
    if (Bring(P1, AtLeast, 2, FIREBAT, LOC_PAD)) {
        has_firebats = 1;
    }
    if (Bring(P1, AtLeast, 2, MARINE, LOC_PAD)) {
        has_marines = 1;
    }
    materials_now = 0;
    if ((has_firebats == 1)) {
        materials_now = 1;
    } else if ((has_marines == 1)) {
        materials_now = 1;
    }
    if ((cooldown == 0)) {
        if (((has_firebats == 1) && Accumulate(P1, AtLeast, R2_COST, Ore))) {
            RemoveUnitAt(2, FIREBAT, LOC_PAD, P1);
            CreateUnit(1, SIEGE_TANK, LOC_PAD, P1);
            SetResources(P1, Subtract, R2_COST, Ore);
            MinimapPing(LOC_PAD);
            fusions2 += 1;
            cooldown = FUSION_COOLDOWN;
        } else if (((has_marines == 1) && Accumulate(P1, AtLeast, R1_COST, Ore))) {
            RemoveUnitAt(2, MARINE, LOC_PAD, P1);
            CreateUnit(1, FIREBAT, LOC_PAD, P1);
            SetResources(P1, Subtract, R1_COST, Ore);
            MinimapPing(LOC_PAD);
            fusions1 += 1;
            cooldown = FUSION_COOLDOWN;
        }
        if (((materials_now == 1) && (prev_materials == 0))) {
            if ((has_firebats == 1)) {
                if (Accumulate(P1, AtMost, (R2_COST - 1), Ore)) {
                    failed_attempts += 1;
                }
            } else if ((has_marines == 1)) {
                if (Accumulate(P1, AtMost, (R1_COST - 1), Ore)) {
                    failed_attempts += 1;
                }
            }
        }
    }
    prev_materials = materials_now;
}
