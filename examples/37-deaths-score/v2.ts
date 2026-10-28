/**
 *
 */

import {
    Add,
    AtLeast,
    CurrentPlayer,
    Custom,
    Deaths,
    DisplayText,
    Exactly,
    Ore,
    Score,
    SetDeaths,
    SetResources,
    SetScore,
    SetTo,
    once,
    session,
} from "eudkit";

export const MARINE = 0;
export const ZERGLING = 37;
export const HYDRALISK = 38;
export const ULTRALISK = 39;

export function main(): void {
    if (Deaths(CurrentPlayer, Exactly, 1, ZERGLING)) {
        SetDeaths(CurrentPlayer, Add, 1, MARINE);
        SetScore(CurrentPlayer, Add, 100, Custom);
    }
    if (Deaths(CurrentPlayer, Exactly, 1, HYDRALISK)) {
        SetDeaths(CurrentPlayer, Add, 2, MARINE);
        SetScore(CurrentPlayer, Add, 250, Custom);
    }
    if (Deaths(CurrentPlayer, Exactly, 1, ULTRALISK)) {
        SetDeaths(CurrentPlayer, Add, 5, MARINE);
        SetScore(CurrentPlayer, Add, 500, Custom);
        DisplayText("Ultralisk down! +500 score.");
    }
    if (Deaths(CurrentPlayer, AtLeast, 5, MARINE)) {
        SetResources(CurrentPlayer, Add, 200, Ore);
        SetDeaths(CurrentPlayer, SetTo, 0, MARINE);
        DisplayText("Kill bonus paid: +200 ore.");
    }
    if (Score(CurrentPlayer, Custom, AtLeast, 1000)) {
        once(() => {
            DisplayText("Veteran! Score passed 1000.");
        });
    }
}
