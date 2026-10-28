/**
 * Scenario 24: Wave Spawner System — Modern eudkit (v2).
 *
 * Ideal Intent:
 * Autonomous defense wave system across 10 progressive waves:
 * 1. Every 240 ticks (10s), launch next wave if no wave is currently streaming.
 * 2. Wave quota scales dynamically: `quota = 4 + 2 * wave`.
 * 3. While wave is active, stream 1 enemy per frame, alternating Zerglings (loc 1)
 *    and Hydralisks (loc 2) for Player 8.
 * 4. When wave quota is filled, mark wave complete.
 * 5. Every 5 completed waves, grant 150 Ore bonus to Player 1.
 * 6. Clearing wave 10 latches victory.
 */

import {
    Add,
    CreateUnit,
    Ore,
    Player1,
    Player8,
    SetResources,
    session,
} from "eudkit";

export const WAVE_PERIOD = 240;
export const MAX_WAVE = 10;
export const BONUS_EVERY = 5;
export const BONUS_ORE = 150;
export let wave = session.var(0, "wave");
export let spawn_timer = session.var(0, "spawn_timer");
export let quota = session.var(0, "quota");
export let spawned_in_wave = session.var(0, "spawned_in_wave");
export let wave_active = session.var(0, "wave_active");
export let spawn_alt = session.var(0, "spawn_alt");
export let total_spawned = session.var(0, "total_spawned");
export let waves_completed = session.var(0, "waves_completed");
export let bonus_counter = session.var(0, "bonus_counter");
export let victory = session.var(0, "victory");

export function main(): void {
    if ((victory == 0)) {
        spawn_timer += 1;
        if (((spawn_timer >= WAVE_PERIOD) && (wave_active == 0) && (wave < MAX_WAVE))) {
            wave += 1;
            quota = (4 + (2 * wave));
            spawned_in_wave = 0;
            wave_active = 1;
            spawn_timer = 0;
        }
        if ((wave_active == 1)) {
            if ((spawn_alt == 0)) {
                CreateUnit(1, "Zerg Zergling", 1, Player8);
                spawn_alt = 1;
            } else {
                CreateUnit(1, "Zerg Hydralisk", 2, Player8);
                spawn_alt = 0;
            }
            spawned_in_wave += 1;
            total_spawned += 1;
            if ((spawned_in_wave >= quota)) {
                wave_active = 0;
                waves_completed += 1;
                bonus_counter += 1;
                if ((bonus_counter >= BONUS_EVERY)) {
                    bonus_counter = 0;
                    SetResources(Player1, Add, BONUS_ORE, Ore);
                }
                if ((wave >= MAX_WAVE)) {
                    victory = 1;
                }
            }
        }
    }
}
