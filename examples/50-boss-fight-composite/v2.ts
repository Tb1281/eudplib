/**
 *
 */

import {
    Add,
    AtLeast,
    CreateUnit,
    CurrentPlayer,
    Deaths,
    Ore,
    P1,
    P8,
    SetResources,
    once,
    session,
} from "eudkit";

export const BOSS_HP_MAX = 1000;
export const ENRAGE_AT = 250;
export const HIT_DAMAGE = 25;
export const MARINE = 0;
export const ZERGLING = 37;
export const HYDRALISK = 38;
export const ULTRALISK = 39;
export const ANYWHERE = 0;
export let phase = session.var(0, "phase");
export let boss_hp = session.var(0, "boss_hp");
export let boss_hits = session.var(0, "boss_hits");
export let applied_hits = session.var(0, "applied_hits");
export let gold = session.var(0, "gold");
export let adds_spawned = session.var(0, "adds_spawned");
export let spawn_timer = session.var(0, "spawn_timer");
export let shown_hp = session.var((-1), "shown_hp");
export let cleanup_timer = session.var(0, "cleanup_timer");
export let bar = session.string(96);

export function main(): void {
    if (Deaths(CurrentPlayer, AtLeast, 1, MARINE)) {
        if ((phase == 0)) {
            phase = 1;
            boss_hp = BOSS_HP_MAX;
            CreateUnit(1, ULTRALISK, ANYWHERE, P8);
        }
    }
    if (((phase == 1) || (phase == 2))) {
        spawn_timer += 1;
        if ((spawn_timer >= 24)) {
            spawn_timer = 0;
            CreateUnit(2, ZERGLING, ANYWHERE, P8);
            adds_spawned += 2;
        }
    }
    if ((boss_hits > applied_hits)) {
        boss_hp -= (HIT_DAMAGE * (boss_hits - applied_hits));
        applied_hits = boss_hits;
    }
    if ((boss_hp <= ENRAGE_AT)) {
        once(() => {
            if ((phase == 1)) {
                phase = 2;
                CreateUnit(4, HYDRALISK, ANYWHERE, P8);
                adds_spawned += 4;
            }
        });
    }
    if (((phase == 1) || (phase == 2))) {
        if ((boss_hp != shown_hp)) {
            shown_hp = boss_hp;
            bar.printf("Boss HP: %d", boss_hp);
        }
        bar.display();
    }
    if ((boss_hp <= 0)) {
        if (((phase == 1) || (phase == 2))) {
            phase = 3;
            gold += 500;
            SetResources(P1, Add, 500, Ore);
            cleanup_timer = 0;
        }
    }
    if ((phase == 3)) {
        cleanup_timer += 1;
        if ((cleanup_timer >= 48)) {
            phase = 0;
            boss_hits = 0;
            applied_hits = 0;
            adds_spawned = 0;
            shown_hp = (-1);
        }
    }
}
