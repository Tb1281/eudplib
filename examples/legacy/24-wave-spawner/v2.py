"""Scenario 24: Wave Spawner System — Modern eudkit (v2).

Ideal Intent:
Autonomous defense wave system across 10 progressive waves:
1. Every 240 ticks (10s), launch next wave if no wave is currently streaming.
2. Wave quota scales dynamically: `quota = 4 + 2 * wave`.
3. While wave is active, stream 1 enemy per frame, alternating Zerglings (loc 1)
   and Hydralisks (loc 2) for Player 8.
4. When wave quota is filled, mark wave complete.
5. Every 5 completed waves, grant 150 Ore bonus to Player 1.
6. Clearing wave 10 latches victory.
"""

import eudkit
from eudkit import (
    session,
    Add,
    CreateUnit,
    Ore,
    Player1,
    Player8,
    SetResources,
)


WAVE_PERIOD = 240
MAX_WAVE = 10
BONUS_EVERY = 5
BONUS_ORE = 150

wave = session.var(0, name="wave")
spawn_timer = session.var(0, name="spawn_timer")
quota = session.var(0, name="quota")
spawned_in_wave = session.var(0, name="spawned_in_wave")
wave_active = session.var(0, name="wave_active")
spawn_alt = session.var(0, name="spawn_alt")
total_spawned = session.var(0, name="total_spawned")
waves_completed = session.var(0, name="waves_completed")
bonus_counter = session.var(0, name="bonus_counter")
victory = session.var(0, name="victory")

@session.fn
def game_loop():
    if victory == 0:
        spawn_timer += 1

        # Launch next wave when timer triggers
        if (spawn_timer >= WAVE_PERIOD) and (wave_active == 0) and (wave < MAX_WAVE):
            wave += 1
            quota = 4 + 2 * wave
            spawned_in_wave = 0
            wave_active = 1
            spawn_timer = 0

        # Stream wave units
        if wave_active == 1:
            if spawn_alt == 0:
                CreateUnit(1, "Zerg Zergling", 1, Player8)
                spawn_alt = 1
            else:
                CreateUnit(1, "Zerg Hydralisk", 2, Player8)
                spawn_alt = 0

            spawned_in_wave += 1
            total_spawned += 1

            # Wave completion check
            if spawned_in_wave >= quota:
                wave_active = 0
                waves_completed += 1
                bonus_counter += 1

                if bonus_counter >= BONUS_EVERY:
                    bonus_counter = 0
                    SetResources(Player1, Add, BONUS_ORE, Ore)

                if wave >= MAX_WAVE:
                    victory = 1
