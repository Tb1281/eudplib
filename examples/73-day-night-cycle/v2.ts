/**
 *
 */

import {
    CreateUnit,
    every,
    session,
    wwrite,
} from "eudkit";

export const ZERGLING = 37;
export const HYDRALISK = 38;
export const CLAWS = 35;
export const WEAPON_DMG_BASE = 0x656EB0;
export const ENEMY = 7;
export const ENEMY_BASE = 50;
export const PHASE_LEN = 600;
export const SPAWN_PERIOD = 100;
export const CLAWS_DAY = 5;
export const CLAWS_NIGHT = 8;
export let phase_tick = session.var(0, "phase_tick");
export let phase = session.var(0, "phase");
export let prev_phase = session.var(0, "prev_phase");
export let day_spawns = session.var(0, "day_spawns");
export let night_spawns = session.var(0, "night_spawns");
export let cycles = session.var(0, "cycles");
export let transitions = session.var(0, "transitions");
export const msg_night = session.map_string("Night falls. The predators grow bold.");
export const msg_dawn = session.map_string("Dawn breaks. The swarm rests.");

export function main(): void {
    phase_tick += 1;
    if ((phase_tick >= PHASE_LEN)) {
        phase_tick = 0;
        if ((phase == 0)) {
            phase = 1;
        } else {
            phase = 0;
        }
    }
    if ((phase != prev_phase)) {
        transitions += 1;
        if ((phase == 1)) {
            wwrite((WEAPON_DMG_BASE + (2 * CLAWS)), CLAWS_NIGHT);
            msg_night.display();
        } else {
            wwrite((WEAPON_DMG_BASE + (2 * CLAWS)), CLAWS_DAY);
            cycles += 1;
            msg_dawn.display();
        }
        prev_phase = phase;
    }
    every(SPAWN_PERIOD, () => {
        if ((phase == 0)) {
            CreateUnit(2, ZERGLING, ENEMY_BASE, ENEMY);
            day_spawns += 2;
        } else {
            CreateUnit(1, HYDRALISK, ENEMY_BASE, ENEMY);
            night_spawns += 1;
        }
    });
}
