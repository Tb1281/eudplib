/**
 * Scenario 28: Boss Phase State Machine — Modern eudkit (v2).
 *
 * Ideal Intent:
 * Autonomous multi-phase boss fight with 3000 initial HP:
 * 1. When Player 1 keeps at least 4 Marines in the boss zone (location 4),
 *    the boss takes 10 damage per loop.
 * 2. Phase 1 (>2000 HP): standard combat phase.
 * 3. Phase 2 (<=2000 HP):
 *    - Entry burst: 4 Hydralisks spawned at location 5, ping spawn zone.
 *    - Periodic: spawns 2 Zerglings every 100 loops.
 * 4. Phase 3 (<=1000 HP):
 *    - Entry burst: 6 Zerglings spawned at location 5, ping spawn zone.
 *    - Periodic: spawns 2 Zerglings + 1 Hydralisk every 60 loops, ping spawn zone.
 * 5. Phase 4 (==0 HP):
 *    - Victory reward: 1000 Ore to Player 1, ping boss zone, latches victory.
 * Modern v2 uses direct variable initialization and clean state transitions,
 * eliminating legacy zero-init workarounds and scratch transition flags.
 */

import {
    Add,
    AtLeast,
    Bring,
    CreateUnit,
    MinimapPing,
    Ore,
    Player1,
    Player8,
    SetResources,
    session,
} from "eudkit";

export const BOSS_MAX_HP = 3000;
export const DAMAGE_PER_LOOP = 10;
export const MARINES_FOR_DAMAGE = 4;
export const LOC_BOSS_ZONE = 4;
export const LOC_SPAWN = 5;
export const P2_PERIOD = 100;
export const P3_PERIOD = 60;
export const DEATH_REWARD_ORE = 1000;
export let boss_hp = session.var(BOSS_MAX_HP, "boss_hp");
export let phase = session.var(1, "phase");
export let phase_timer = session.var(0, "phase_timer");
export let minion_spawns = session.var(0, "minion_spawns");
export let victory = session.var(0, "victory");

export function main(): void {
    if ((victory == 0)) {
        if (Bring(Player1, AtLeast, MARINES_FOR_DAMAGE, "Terran Marine", LOC_BOSS_ZONE)) {
            if ((boss_hp >= 1)) {
                boss_hp -= DAMAGE_PER_LOOP;
            }
        }
        if (((phase == 1) && (boss_hp <= 2000))) {
            phase = 2;
            phase_timer = 0;
            CreateUnit(4, "Zerg Hydralisk", LOC_SPAWN, Player8);
            MinimapPing(LOC_SPAWN);
        }
        if (((phase == 2) && (boss_hp <= 1000))) {
            phase = 3;
            phase_timer = 0;
            CreateUnit(6, "Zerg Zergling", LOC_SPAWN, Player8);
            MinimapPing(LOC_SPAWN);
        }
        if (((phase == 3) && (boss_hp == 0))) {
            phase = 4;
            SetResources(Player1, Add, DEATH_REWARD_ORE, Ore);
            MinimapPing(LOC_BOSS_ZONE);
            victory = 1;
        }
        if ((phase == 2)) {
            phase_timer += 1;
            if ((phase_timer >= P2_PERIOD)) {
                phase_timer = 0;
                CreateUnit(2, "Zerg Zergling", LOC_SPAWN, Player8);
                minion_spawns += 2;
            }
        }
        if ((phase == 3)) {
            phase_timer += 1;
            if ((phase_timer >= P3_PERIOD)) {
                phase_timer = 0;
                CreateUnit(2, "Zerg Zergling", LOC_SPAWN, Player8);
                CreateUnit(1, "Zerg Hydralisk", LOC_SPAWN, Player8);
                MinimapPing(LOC_SPAWN);
                minion_spawns += 3;
            }
        }
    }
}
