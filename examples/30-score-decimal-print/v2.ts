/**
 * Scenario 30: Decimal Scoreboard Print — Modern eudkit (v2).
 *
 * Ideal Intent:
 * Track Player 1's score from two deaths counters:
 * - Player 8 Zergling deaths = kills scored (`KILLS_ADDR`)
 * - Player 1 Marine deaths = deaths suffered (`DEATHS_ADDR`)
 * Every 120 trigger loops:
 * 1. Score = max(0, kills * 100 - deaths * 30).
 * 2. Accuracy = (kills * 100) // (kills + deaths) if kills + deaths > 0 else 0.
 * 3. Composes and displays formatted decimal status line:
 *    `"SCORE: %d  KILLS: %d  DEATHS: %d  ACC: %d%%"`
 * Modern v2 uses native integer floor division `//`, declarative timing `with every(120):`,
 * and unified `session.string()` formatting.
 */

import {
    EPD,
    dwread_epd,
    every,
    session,
} from "eudkit";

export const KILLS_ADDR = ((0x58A364 + (4 * 7)) + (48 * 37));
export const DEATHS_ADDR = 0x58A364;
export const PRINT_PERIOD = 120;
export const SCORE_PER_KILL = 100;
export const PENALTY_PER_DEATH = 30;
export let board_buf = session.string(128);
export let prev_kills = session.var(0, "prev_kills");
export let prev_deaths = session.var(0, "prev_deaths");
export let kills_total = session.var(0, "kills_total");
export let deaths_total = session.var(0, "deaths_total");
export let score_base = session.var(0, "score_base");
export let score_penalty = session.var(0, "score_penalty");
export let score = session.var(0, "score");
export let accuracy = session.var(0, "accuracy");
export let prints = session.var(0, "prints");

export function main(): void {
    let cur_k = dwread_epd(EPD(KILLS_ADDR));
    if ((cur_k > prev_kills)) {
        kills_total += (cur_k - prev_kills);
    }
    prev_kills = cur_k;
    let cur_d = dwread_epd(EPD(DEATHS_ADDR));
    if ((cur_d > prev_deaths)) {
        deaths_total += (cur_d - prev_deaths);
    }
    prev_deaths = cur_d;
    score_base = (kills_total * SCORE_PER_KILL);
    score_penalty = (deaths_total * PENALTY_PER_DEATH);
    if ((score_base >= score_penalty)) {
        score = (score_base - score_penalty);
    } else {
        score = 0;
    }
    every(PRINT_PERIOD, () => {
        let total_events = (kills_total + deaths_total);
        if ((total_events >= 1)) {
            accuracy = (((kills_total * 100) / total_events) | 0);
        } else {
            accuracy = 0;
        }
        board_buf.printf("SCORE: %d  KILLS: %d  DEATHS: %d  ACC: %d%%", score, kills_total, deaths_total, accuracy);
        board_buf.display();
        prints += 1;
    });
}
