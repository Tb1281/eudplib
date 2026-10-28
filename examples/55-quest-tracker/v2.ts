/**
 *
 */

import {
    Add,
    Gas,
    MinimapPing,
    Ore,
    P1,
    P8,
    SetResources,
    dwread,
    every,
    session,
} from "eudkit";

export const P1 = 0;
export const P8 = 7;
export const LOC_BASE = 1;
export const ZERGLING = 37;
export const HYDRALISK = 38;
export const ULTRALISK = 39;
export const LOG_PERIOD = 48;
export const DEATHS_ZERGLING = ((0x58A364 + (4 * P8)) + (48 * ZERGLING));
export const DEATHS_HYDRALISK = ((0x58A364 + (4 * P8)) + (48 * HYDRALISK));
export const DEATHS_ULTRALISK = ((0x58A364 + (4 * P8)) + (48 * ULTRALISK));
export let quest = session.var(1, "quest");
export let progress = session.var(0, "progress");
export let campaign_done = session.var(0, "campaign_done");
export let completions = session.var(0, "completions");
export let total_kills = session.var(0, "total_kills");
export let prev_z = session.var(0, "prev_z");
export let prev_h = session.var(0, "prev_h");
export let prev_u = session.var(0, "prev_u");
export let quest_log = session.string(96);

export function main(): void {
    if ((campaign_done == 0)) {
        let cur_z = dwread(DEATHS_ZERGLING);
        let cur_h = dwread(DEATHS_HYDRALISK);
        let cur_u = dwread(DEATHS_ULTRALISK);
        if ((quest == 1)) {
            if ((cur_z > prev_z)) {
                progress += (cur_z - prev_z);
                total_kills += (cur_z - prev_z);
            }
        } else if ((quest == 2)) {
            if ((cur_h > prev_h)) {
                progress += (cur_h - prev_h);
                total_kills += (cur_h - prev_h);
            }
        } else if ((quest == 3)) {
            if ((cur_u > prev_u)) {
                progress += (cur_u - prev_u);
                total_kills += (cur_u - prev_u);
            }
        }
        prev_z = cur_z;
        prev_h = cur_h;
        prev_u = cur_u;
        if ((quest == 1)) {
            if ((progress >= 10)) {
                SetResources(P1, Add, 150, Ore);
                MinimapPing(LOC_BASE);
                progress = 0;
                quest = 2;
                completions += 1;
            }
        }
        if ((quest == 2)) {
            if ((progress >= 5)) {
                SetResources(P1, Add, 250, Ore);
                SetResources(P1, Add, 50, Gas);
                MinimapPing(LOC_BASE);
                progress = 0;
                quest = 3;
                completions += 1;
            }
        }
        if ((quest == 3)) {
            if ((progress >= 2)) {
                SetResources(P1, Add, 500, Ore);
                MinimapPing(LOC_BASE);
                progress = 0;
                quest = 4;
                completions += 1;
                campaign_done = 1;
            }
        }
    }
    every(LOG_PERIOD, () => {
        if ((quest == 1)) {
            quest_log.printf("QUEST 1: %d/10", progress);
        } else if ((quest == 2)) {
            quest_log.printf("QUEST 2: %d/5", progress);
        } else if ((quest == 3)) {
            quest_log.printf("QUEST 3: %d/2", progress);
        } else if ((quest == 4)) {
            quest_log.printf("QUEST 4: DONE");
        }
        quest_log.display();
    });
}
