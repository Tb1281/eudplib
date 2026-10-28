/**
 *
 */

import {
    dwread,
    every,
    session,
} from "eudkit";

export const CUSTOM_SCORE_ADDR = 0x5822F4;
export const REFRESH_PERIOD = 240;
export const PLAYERS = 8;
export const line_bufs = [session.var(0, "line_bufs_0"), session.var(0, "line_bufs_1"), session.var(0, "line_bufs_2")];
export const scores = [session.var(0, "scores_0"), session.var(0, "scores_1"), session.var(0, "scores_2"), session.var(0, "scores_3"), session.var(0, "scores_4"), session.var(0, "scores_5"), session.var(0, "scores_6"), session.var(0, "scores_7")];
export const ranks = [session.var(0, "ranks_0"), session.var(0, "ranks_1"), session.var(0, "ranks_2"), session.var(0, "ranks_3"), session.var(0, "ranks_4"), session.var(0, "ranks_5"), session.var(0, "ranks_6"), session.var(0, "ranks_7")];
export let refreshes = session.var(0, "refreshes");
export let leader = session.var(0, "leader");
export const shown = [session.var(0, "shown_0"), session.var(0, "shown_1"), session.var(0, "shown_2")];

export function main(): void {
    every(REFRESH_PERIOD, () => {
        for (let p = 0; p < PLAYERS; p++) {
            scores[p] = dwread((CUSTOM_SCORE_ADDR + (4 * p)));
        }
        for (let p = 0; p < PLAYERS; p++) {
            ranks[p] = 1;
            for (let q = 0; q < PLAYERS; q++) {
                if ((q != p)) {
                    if ((scores[q] > scores[p])) {
                        ranks[p] += 1;
                    }
                }
            }
        }
        leader = 0;
        for (let p = 0; p < PLAYERS; p++) {
            if (((ranks[p] == 1) && (leader == 0))) {
                leader = (p + 1);
            }
        }
        for (let r = 0; r < 3; r++) {
            shown[r] = 0;
            for (let p = 0; p < PLAYERS; p++) {
                if ((ranks[p] == (r + 1))) {
                    line_bufs[r].printf("RANK ", (r + 1), ": P", (p + 1), " - ", scores[p]);
                    shown[r] = 1;
                }
            }
            if ((shown[r] == 1)) {
                line_bufs[r].display();
            }
        }
        refreshes += 1;
    });
}
