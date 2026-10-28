"""LEGACY wave spawner (benchmark 24).

Same intent as ours.js: a single-player defense wave system.
Every trigger loop a spawn timer advances; every 240 loops the next
wave starts (waves 1..10) with a quota of 4 + 2*wave units. While a
wave is active, one unit per loop streams in, alternating
Zergling at location 1 / Hydralisk at location 2 for Player 8.
Every 5 completed waves pay Player 1 a 150-ore bonus. Clearing
wave 10 latches the victory flag.
"""

from eudkit import *

WAVE_PERIOD = 240        # loops between wave starts
MAX_WAVE = 10
BONUS_EVERY = 5          # waves per bonus payout
BONUS_ORE = 150
LOC_SPAWN_A = 1          # location ids (numeric: no basemap lookup)
LOC_SPAWN_B = 2
UNIT_ZERGLING = 37       # unit ids (numeric form, as in the pilot)
UNIT_HYDRALISK = 34

wave = EUDVariable()           # current wave number (0 = none yet)
spawn_timer = EUDVariable()    # loops since the last wave started
quota = EUDVariable()          # units to spawn in the current wave
spawned_in_wave = EUDVariable()  # units spawned so far this wave
wave_active = EUDVariable()    # 1 while the wave is still streaming in
spawn_alt = EUDVariable()      # 0/1 spawn parity toggle
total_spawned = EUDVariable()  # lifetime units spawned
waves_completed = EUDVariable()  # waves fully spawned
bonus_counter = EUDVariable()  # completed waves since the last bonus
victory = EUDVariable()       # latched once wave 10 is fully spawned


def run():
    global wave, spawn_timer, quota, spawned_in_wave, wave_active
    global spawn_alt, total_spawned, waves_completed, bonus_counter
    global victory
    if EUDIf()(victory == 0):
        spawn_timer += 1

        # Wave start: timer expired, no wave streaming, waves remain.
        if EUDIf()([spawn_timer >= WAVE_PERIOD, wave_active == 0,
                    wave < MAX_WAVE]):
            wave += 1
            quota << 4 + 2 * wave
            spawned_in_wave << 0
            wave_active << 1
            spawn_timer << 0
            DoActions(MinimapPing(LOC_SPAWN_A))
        EUDEndIf()

        # Streaming: one unit per loop while the wave is active.
        if EUDIf()(wave_active == 1):
            if EUDIf()(spawn_alt == 0):
                DoActions(CreateUnit(1, UNIT_ZERGLING, LOC_SPAWN_A, Player8))
                spawn_alt << 1
            if EUDElse()():
                DoActions(CreateUnit(1, UNIT_HYDRALISK, LOC_SPAWN_B, Player8))
                spawn_alt << 0
            EUDEndIf()
            spawned_in_wave += 1
            total_spawned += 1

            # Wave finished streaming.
            if EUDIf()(spawned_in_wave >= quota):
                wave_active << 0
                waves_completed += 1
                bonus_counter += 1
                if EUDIf()(bonus_counter >= BONUS_EVERY):
                    bonus_counter << 0
                    DoActions(SetResources(Player1, Add, BONUS_ORE, Ore))
                EUDEndIf()
                if EUDIf()(wave >= MAX_WAVE):
                    victory << 1
                EUDEndIf()
            EUDEndIf()
        EUDEndIf()
    EUDEndIf()
