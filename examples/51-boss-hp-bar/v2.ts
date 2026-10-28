/**
 *
 */

import {
    Add,
    AtLeast,
    Bring,
    CreateUnit,
    MinimapPing,
    Ore,
    P1,
    P8,
    SetResources,
    every,
    session,
} from "eudkit";

export const P1 = 0;
export const P8 = 7;
export const MARINE = 0;
export const ZERGLING = 37;
export const HYDRALISK = 38;
export const LOC_BOSS = 1;
export const LOC_SPAWN = 2;
export const BOSS_MAX_HP = 5000;
export const DAMAGE = 25;
export const PHASE2_HP = 3000;
export const PHASE3_HP = 1500;
export const REWARD_ORE = 800;
export const BAR_PERIOD = 12;
export const BAR_SEGMENTS = 10;
export const BAR_STRINGS = ["----------", "#---------", "##--------", "###-------", "####------", "#####-----", "######----", "#######---", "########--", "#########-", "##########"];
export let boss_hp = session.var(BOSS_MAX_HP, "boss_hp");
export let phase = session.var(1, "phase");
export let victory = session.var(0, "victory");
export let bursts = session.var(0, "bursts");
export let renders = session.var(0, "renders");
export let bar_buf = session.string(64);

export function main(): void {
    if ((victory == 0)) {
        if (Bring(P1, AtLeast, 3, MARINE, LOC_BOSS)) {
            if ((boss_hp >= DAMAGE)) {
                boss_hp -= DAMAGE;
            }
        }
        if ((phase == 1)) {
            if ((boss_hp <= PHASE2_HP)) {
                if ((boss_hp >= 1)) {
                    phase = 2;
                    CreateUnit(4, ZERGLING, LOC_SPAWN, P8);
                    MinimapPing(LOC_SPAWN);
                    bursts += 1;
                }
            }
        }
        if ((phase == 2)) {
            if ((boss_hp <= PHASE3_HP)) {
                if ((boss_hp >= 1)) {
                    phase = 3;
                    CreateUnit(4, ZERGLING, LOC_SPAWN, P8);
                    CreateUnit(2, HYDRALISK, LOC_SPAWN, P8);
                    MinimapPing(LOC_SPAWN);
                    bursts += 1;
                }
            }
        }
        if ((boss_hp == 0)) {
            victory = 1;
            SetResources(P1, Add, REWARD_ORE, Ore);
            MinimapPing(LOC_BOSS);
        }
    }
    every(BAR_PERIOD, () => {
        let filled = (((boss_hp * BAR_SEGMENTS) / BOSS_MAX_HP) | 0);
        for (let i = 0; i < (BAR_SEGMENTS + 1); i++) {
            if ((filled == i)) {
                bar_buf.printf((("BOSS [" + BAR_STRINGS[i]) + "] %d"), boss_hp);
            }
        }
        bar_buf.display();
        renders += 1;
    });
}
