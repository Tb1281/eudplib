/**
 *
 */

import {
    AtLeast,
    Bring,
    DisplayText,
    MinimapPing,
    P2,
    session,
} from "eudkit";

export const COOLDOWN_FRAMES = 168;
export const ENEMY = P2;
export const MARINE = 0;
export const HYDRALISK = 38;
export const ZERGLING = 37;
export const WATCH_ZONE = 1;
export const GATE_ZONE = 2;
export const REAR_ZONE = 3;
export let watch_cd = session.var(0, "watch_cd");
export let gate_cd = session.var(0, "gate_cd");
export let rear_cd = session.var(0, "rear_cd");

export function main(): void {
    if ((watch_cd >= 1)) {
        watch_cd -= 1;
    }
    if ((gate_cd >= 1)) {
        gate_cd -= 1;
    }
    if ((rear_cd >= 1)) {
        rear_cd -= 1;
    }
    if (Bring(ENEMY, AtLeast, 1, MARINE, WATCH_ZONE)) {
        if ((watch_cd == 0)) {
            watch_cd = COOLDOWN_FRAMES;
            MinimapPing(WATCH_ZONE);
            DisplayText("Warning: enemy Marine in the Watch Zone!");
        }
    }
    if (Bring(ENEMY, AtLeast, 1, HYDRALISK, GATE_ZONE)) {
        if ((gate_cd == 0)) {
            gate_cd = COOLDOWN_FRAMES;
            MinimapPing(GATE_ZONE);
            DisplayText("Warning: enemy Hydralisk at the Gate Zone!");
        }
    }
    if (Bring(ENEMY, AtLeast, 1, ZERGLING, REAR_ZONE)) {
        if ((rear_cd == 0)) {
            rear_cd = COOLDOWN_FRAMES;
            MinimapPing(REAR_ZONE);
            DisplayText("Warning: enemy Zergling in the Rear Zone!");
        }
    }
}
