/**
 *
 */

import {
    DisplayText,
    session,
} from "eudkit";

export let hero_atk = session.var(50, "hero_atk");
export let hero_def = session.var(20, "hero_def");
export let last_damage = session.var(0, "last_damage");
export let last_dist2 = session.var(0, "last_dist2");
export let critical = session.var(0, "critical");
export let hit_count = session.var(0, "hit_count");
export let bonus_damage = session.var(0, "bonus_damage");

function calc_damage(atk: number, defense: number): number {
    let dmg = 1;
    if ((atk > defense)) {
        dmg = (atk - defense);
    }
    return dmg;
}

function dist2(x1: number, y1: number, x2: number, y2: number): number {
    let dx = (x1 - x2);
    let dy = (y1 - y2);
    let res = 0;
    res = ((dx * dx) + (dy * dy));
    return res;
}

export function main(): void {
    hero_atk = 50;
    hero_def = 20;
    last_damage = calc_damage(hero_atk, hero_def);
    last_dist2 = dist2(100, 100, 160, 130);
    if ((last_damage > 30)) {
        critical = 1;
        hit_count += 1;
    } else {
        critical = 0;
    }
    bonus_damage = calc_damage(80, 15);
    if ((bonus_damage >= 60)) {
        hero_atk += 5;
    }
    if ((last_dist2 <= 10000)) {
        DisplayText("In range");
    }
}
