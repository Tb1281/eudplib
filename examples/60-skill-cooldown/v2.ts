/**
 *
 */

import {
    Clear,
    KillUnitAt,
    MinimapPing,
    P8,
    Set,
    SetSwitch,
    Switch,
    session,
} from "eudkit";

export const P8 = 7;
export const ZERGLING = 37;
export const HYDRALISK = 38;
export const LOC_NOVA = 5;
export const LOC_HERO = 6;
export const MANA_MAX = 100;
export const REGEN_READY = 3;
export const REGEN_COOLING = 1;
export const NOVA_COOLDOWN = 240;
export const NOVA_ZERGLINGS = 4;
export const NOVA_HYDRALISKS = 2;
export const SW_NOVA = 8;
export let mana = session.var(0, "mana");
export let cooldown = session.var(0, "cooldown");
export let ready_flag = session.var(0, "ready_flag");
export let casts = session.var(0, "casts");
export let failed_casts = session.var(0, "failed_casts");
export let total_mana_spent = session.var(0, "total_mana_spent");

export function main(): void {
    if ((cooldown >= 1)) {
        cooldown -= 1;
    }
    if ((cooldown == 0)) {
        if ((mana <= (MANA_MAX - REGEN_READY))) {
            mana += REGEN_READY;
        } else if ((mana < MANA_MAX)) {
            mana = MANA_MAX;
        }
    } else if ((mana < MANA_MAX)) {
        mana += REGEN_COOLING;
    }
    if (Switch(SW_NOVA, Set)) {
        SetSwitch(SW_NOVA, Clear);
        if (((mana == MANA_MAX) && (cooldown == 0))) {
            KillUnitAt(NOVA_ZERGLINGS, ZERGLING, LOC_NOVA, P8);
            KillUnitAt(NOVA_HYDRALISKS, HYDRALISK, LOC_NOVA, P8);
            MinimapPing(LOC_NOVA);
            mana = 0;
            cooldown = NOVA_COOLDOWN;
            ready_flag = 0;
            casts += 1;
            total_mana_spent += MANA_MAX;
        } else {
            failed_casts += 1;
        }
    }
    if (((cooldown == 0) && (mana == MANA_MAX))) {
        if ((ready_flag == 0)) {
            MinimapPing(LOC_HERO);
            ready_flag = 1;
        }
    }
}
