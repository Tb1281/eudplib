/**
 *
 */

import {
    AtLeast,
    Command,
    Exactly,
    KillUnit,
    session,
} from "eudkit";

export const COMMAND_CENTER = 106;
export const ANY_UNIT = 229;
export const started = [session.var(0, "started_0"), session.var(0, "started_1"), session.var(0, "started_2"), session.var(0, "started_3")];
export const out = [session.var(0, "out_0"), session.var(0, "out_1"), session.var(0, "out_2"), session.var(0, "out_3")];
export let alive_count = session.var(0, "alive_count");
export let winner = session.var(0, "winner");
export let game_over = session.var(0, "game_over");
export const msg_eliminated = session.map_string("A command center has fallen - a player is eliminated.");
export const msg_victory = session.map_string("One player remains - victory!");

export function main(): void {
    for (let p = 0; p < 4; p++) {
        if (((started[p] == 0) && Command(p, AtLeast, 1, COMMAND_CENTER))) {
            started[p] = 1;
            alive_count += 1;
        }
        if (((started[p] == 1) && (out[p] == 0) && Command(p, Exactly, 0, COMMAND_CENTER))) {
            out[p] = 1;
            alive_count -= 1;
            KillUnit(ANY_UNIT, p);
            msg_eliminated.display();
        }
    }
    if (((game_over == 0) && (alive_count == 1))) {
        for (let p = 0; p < 4; p++) {
            if (((started[p] == 1) && (out[p] == 0))) {
                winner = (p + 1);
            }
        }
        msg_victory.display();
        game_over = 1;
    }
}
