/**
 *
 */

import {
    AtLeast,
    CreateUnit,
    CurrentPlayer,
    Deaths,
    Defeat,
    DisplayText,
    P8,
    Victory,
    session,
} from "eudkit";

export const PH_IDLE = 0;
export const PH_COUNTDOWN = 1;
export const PH_WAVE_LIGHT = 2;
export const PH_WAVE_HEAVY = 3;
export const PH_BOSS = 4;
export const PH_SHOP = 5;
export const PH_VICTORY = 6;
export const PH_DEFEAT = 7;
export const MARINE = 0;
export const ZERGLING = 37;
export const HYDRALISK = 38;
export const ULTRALISK = 39;
export const COMMAND_CENTER = 106;
export const ANYWHERE = 0;
export let phase = session.var(0, "phase");
export let wave_timer = session.var(0, "wave_timer");
export let wave_number = session.var(0, "wave_number");
export let spawn_left = session.var(0, "spawn_left");
export let gold = session.var(0, "gold");
export let kills = session.var(0, "kills");

export function main(): void {
    if ((phase == PH_IDLE)) {
        if (Deaths(CurrentPlayer, AtLeast, 1, MARINE)) {
            phase = PH_COUNTDOWN;
            wave_timer = 72;
        }
    } else if ((phase == PH_COUNTDOWN)) {
        wave_timer -= 1;
        if ((wave_timer == 0)) {
            wave_number += 1;
            spawn_left = (10 + (wave_number * 2));
            phase = PH_WAVE_LIGHT;
        }
    } else if ((phase == PH_WAVE_LIGHT)) {
        if ((spawn_left > 0)) {
            CreateUnit(1, ZERGLING, ANYWHERE, P8);
            spawn_left -= 1;
        }
        if ((spawn_left == 0)) {
            phase = PH_WAVE_HEAVY;
            spawn_left = wave_number;
        }
    } else if ((phase == PH_WAVE_HEAVY)) {
        if ((spawn_left > 0)) {
            CreateUnit(1, HYDRALISK, ANYWHERE, P8);
            spawn_left -= 1;
        }
        if ((spawn_left == 0)) {
            if ((wave_number >= 5)) {
                phase = PH_BOSS;
            } else {
                phase = PH_SHOP;
            }
        }
    } else if ((phase == PH_BOSS)) {
        if (Deaths(P8, AtLeast, 1, ULTRALISK)) {
            gold += 500;
            phase = PH_SHOP;
        }
    } else if ((phase == PH_SHOP)) {
        wave_timer = 240;
        phase = PH_COUNTDOWN;
    } else if ((phase == PH_VICTORY)) {
        DisplayText("Victory!");
        Victory();
    } else if ((phase == PH_DEFEAT)) {
        DisplayText("Defeat...");
        Defeat();
    } else {
        phase = PH_IDLE;
    }
    if ((kills >= 300)) {
        phase = PH_VICTORY;
    }
    if (Deaths(CurrentPlayer, AtLeast, 1, COMMAND_CENTER)) {
        phase = PH_DEFEAT;
    }
}
