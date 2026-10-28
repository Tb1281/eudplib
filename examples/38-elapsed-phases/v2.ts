/**
 *
 */

import {
    AtLeast,
    CreateUnit,
    DisplayText,
    ElapsedTime,
    P2,
    once,
    session,
} from "eudkit";

export const MARINE = 0;
export const ZERGLING = 37;
export const HYDRALISK = 38;
export const ULTRALISK = 39;
export const SPAWN = 1;
export const SPAWN_B = 2;

export function main(): void {
    if (ElapsedTime(AtLeast, 10)) {
        once(() => {
            DisplayText("Get ready - first wave in 20 seconds.");
        });
    }
    if (ElapsedTime(AtLeast, 30)) {
        once(() => {
            DisplayText("Wave 1 incoming!");
            CreateUnit(4, MARINE, SPAWN, P2);
        });
    }
    if (ElapsedTime(AtLeast, 90)) {
        once(() => {
            DisplayText("Wave 2 incoming!");
            CreateUnit(4, ZERGLING, SPAWN, P2);
            CreateUnit(4, ZERGLING, SPAWN_B, P2);
        });
    }
    if (ElapsedTime(AtLeast, 180)) {
        once(() => {
            DisplayText("WARNING: something huge approaches...");
            CreateUnit(1, ULTRALISK, SPAWN, P2);
            CreateUnit(4, HYDRALISK, SPAWN_B, P2);
        });
    }
    if (ElapsedTime(AtLeast, 240)) {
        once(() => {
            DisplayText("Final wave! Hold the line!");
            CreateUnit(6, HYDRALISK, SPAWN, P2);
            CreateUnit(6, HYDRALISK, SPAWN_B, P2);
        });
    }
}
