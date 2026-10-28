/**
 *
 */

import {
    Add,
    AtLeast,
    Bring,
    CreateUnit,
    Ore,
    SetResources,
    session,
} from "eudkit";

export const CIVILIAN = 15;
export const MARINE = 0;
export const ULTRALISK = 39;
export const ZONE_A = 10;
export const ZONE_B = 11;
export const ZONE_C = 12;
export const BASE0 = 30;
export const CENTER = 40;
export const ENEMY = 7;
export const WINDOW = 360;
export const ARMY_PER_PLAYER = 4;
export const WEALTH_ORE = 300;
export let tick = session.var(0, "tick");
export const votes = [session.var(0, "votes_0"), session.var(0, "votes_1"), session.var(0, "votes_2"), session.var(0, "votes_3")];
export let cnt_a = session.var(0, "cnt_a");
export let cnt_b = session.var(0, "cnt_b");
export let cnt_c = session.var(0, "cnt_c");
export let votes_cast = session.var(0, "votes_cast");
export let winner = session.var(0, "winner");
export let resolved = session.var(0, "resolved");
export const msg_army = session.map_string("Vote result: ARMY - reinforcements for all.");
export const msg_wealth = session.map_string("Vote result: WEALTH - 300 ore for all.");
export const msg_challenge = session.map_string("Vote result: CHALLENGE - good luck.");
export const msg_none = session.map_string("Vote closed: no votes were cast.");

export function main(): void {
    if ((resolved == 0)) {
        tick += 1;
        for (let p = 0; p < 4; p++) {
            if (Bring(p, AtLeast, 1, CIVILIAN, ZONE_A)) {
                votes[p] = 1;
            } else if (Bring(p, AtLeast, 1, CIVILIAN, ZONE_B)) {
                votes[p] = 2;
            } else if (Bring(p, AtLeast, 1, CIVILIAN, ZONE_C)) {
                votes[p] = 3;
            }
        }
        if ((tick >= WINDOW)) {
            for (let p = 0; p < 4; p++) {
                if ((votes[p] == 1)) {
                    cnt_a += 1;
                    votes_cast += 1;
                } else if ((votes[p] == 2)) {
                    cnt_b += 1;
                    votes_cast += 1;
                } else if ((votes[p] == 3)) {
                    cnt_c += 1;
                    votes_cast += 1;
                }
            }
            if (((votes_cast >= 1) && (cnt_a >= cnt_b) && (cnt_a >= cnt_c))) {
                winner = 1;
            } else if (((votes_cast >= 1) && (cnt_b > cnt_a) && (cnt_b >= cnt_c))) {
                winner = 2;
            } else if ((votes_cast >= 1)) {
                winner = 3;
            }
            if ((winner == 1)) {
                for (let p = 0; p < 4; p++) {
                    CreateUnit(ARMY_PER_PLAYER, MARINE, (BASE0 + p), p);
                }
                msg_army.display();
            } else if ((winner == 2)) {
                for (let p = 0; p < 4; p++) {
                    SetResources(p, Add, WEALTH_ORE, Ore);
                }
                msg_wealth.display();
            } else if ((winner == 3)) {
                CreateUnit(1, ULTRALISK, CENTER, ENEMY);
                msg_challenge.display();
            } else if ((votes_cast == 0)) {
                msg_none.display();
            }
            resolved = 1;
        }
    }
}
