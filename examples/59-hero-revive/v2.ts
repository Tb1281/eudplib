/**
 *
 */

import {
    Accumulate,
    AtLeast,
    Command,
    CreateUnit,
    Exactly,
    MinimapPing,
    Ore,
    P1,
    SetResources,
    Subtract,
    session,
} from "eudkit";

export const P1 = 0;
export const MARINE = 0;
export const LOC_BASE = 1;
export const BASE_WAIT = 180;
export const WAIT_PER_DEATH = 30;
export const REVIVE_COST = 100;
export let hero_state = session.var(0, "hero_state");
export let countdown = session.var(0, "countdown");
export let waiting = session.var(0, "waiting");
export let deaths = session.var(0, "deaths");
export let revives = session.var(0, "revives");
export let total_dead_loops = session.var(0, "total_dead_loops");
export let total_fees = session.var(0, "total_fees");
export const msg_fallen = session.map_string("Hero has fallen");
export const msg_returned = session.map_string("Hero has returned");

export function main(): void {
    if ((hero_state == 0)) {
        if (Command(P1, Exactly, 0, MARINE)) {
            hero_state = 1;
            deaths += 1;
            countdown = (BASE_WAIT + (deaths * WAIT_PER_DEATH));
            MinimapPing(LOC_BASE);
            msg_fallen.display();
        }
    }
    if ((hero_state == 1)) {
        total_dead_loops += 1;
        if ((countdown >= 1)) {
            countdown -= 1;
        }
        if ((countdown == 0)) {
            if (Accumulate(P1, AtLeast, REVIVE_COST, Ore)) {
                SetResources(P1, Subtract, REVIVE_COST, Ore);
                CreateUnit(1, MARINE, LOC_BASE, P1);
                MinimapPing(LOC_BASE);
                msg_returned.display();
                total_fees += REVIVE_COST;
                hero_state = 0;
                waiting = 0;
                revives += 1;
            } else {
                waiting = 1;
            }
        }
    }
}
