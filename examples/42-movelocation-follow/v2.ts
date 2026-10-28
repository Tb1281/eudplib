/**
 *
 */

import {
    Add,
    AtLeast,
    Bring,
    CurrentPlayer,
    Custom,
    DisplayText,
    MinimapPing,
    MoveLocation,
    P2,
    SetScore,
    once,
    session,
} from "eudkit";

export const MARINE = 0;
export const ZERGLING = 37;
export const HYDRALISK = 38;
export const TRACK_ZONE = 1;
export const BASE_ZONE = 2;
export const ANYWHERE = 0;
export let score_cd = session.var(0, "score_cd");

export function main(): void {
    MoveLocation(TRACK_ZONE, MARINE, CurrentPlayer, ANYWHERE);
    if ((score_cd >= 1)) {
        score_cd -= 1;
    }
    if (Bring(CurrentPlayer, AtLeast, 3, ZERGLING, TRACK_ZONE)) {
        once(() => {
            DisplayText("Contact! Zerglings on our Marine.");
        });
        if ((score_cd == 0)) {
            score_cd = 84;
            SetScore(CurrentPlayer, Add, 10, Custom);
        }
    }
    if (Bring(P2, AtLeast, 1, HYDRALISK, BASE_ZONE)) {
        once(() => {
            MinimapPing(BASE_ZONE);
            DisplayText("A Hydralisk slipped into the Base Zone!");
        });
    }
    if (Bring(P2, AtLeast, 5, ZERGLING, BASE_ZONE)) {
        once(() => {
            MinimapPing(BASE_ZONE);
            DisplayText("The base is being overrun!");
        });
    }
}
