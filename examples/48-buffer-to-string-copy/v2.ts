/**
 *
 */

import {
    Kills,
    session,
} from "eudkit";

export let status_buf = session.string(64);
export let wave = session.var(0, "wave");
export let kills = session.var(0, "kills");
export let gold = session.var(0, "gold");
export let wave_timer = session.var(0, "wave_timer");
export let filled_wave = session.var((-1), "filled_wave");
export let filled_kills = session.var((-1), "filled_kills");
export let filled_gold = session.var((-1), "filled_gold");
export let filled_timer = session.var((-1), "filled_timer");
export let copies_done = session.var(0, "copies_done");
export let line_version = session.var(0, "line_version");

export function main(): void {
    let stale = 0;
    if ((filled_wave != wave)) {
        stale = 1;
    }
    if ((filled_kills != kills)) {
        stale = 1;
    }
    if ((filled_gold != gold)) {
        stale = 1;
    }
    if ((filled_timer != wave_timer)) {
        stale = 1;
    }
    if ((stale == 1)) {
        status_buf.printf("Wave %d | Kills %d | Gold %d | Timer %d", wave, kills, gold, wave_timer);
        status_buf.display();
        filled_wave = wave;
        filled_kills = kills;
        filled_gold = gold;
        filled_timer = wave_timer;
        copies_done += 1;
        line_version += 1;
    }
}
