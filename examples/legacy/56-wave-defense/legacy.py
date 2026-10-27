# 56 - wave defense, full loop (legacy eudplib)
#
# Intent: a complete single-player wave-defense game loop for
# Player 1 against Player 8, as a two-state machine:
#   INTERMISSION (120 loops) -> wave starts: quota = 6 + 3*wave
#   units (waves 1..8), a minimap ping, state = ASSAULT.
#   ASSAULT: one unit streams in every 6 loops at the spawn point
#   (location 1), alternating Zergling / Hydralisk. Living enemy
#   count is tracked exactly: +1 per spawn, minus the Deaths-table
#   deltas of Player 8's two unit types each loop. A unit reaching
#   the base (location 2) leaks: it is killed on the spot (its
#   death registers in next loop's delta, keeping `alive` exact),
#   lives -1. When nothing is left to spawn and nothing is alive,
#   the wave is cleared: +150 ore, +75 gas every third wave, next
#   wave's intermission begins. Clearing wave 8 latches victory;
#   lives reaching 0 latches defeat.
#
# Counting form (pilot convention): run() builds the scenario's
# triggers once; benchmarks/pilot/count_legacy.py calls it inside a
# fresh PushTriggerScope and reports the GetTriggerCounter() delta.
# In production the body is the beforeTriggerExec hook; run() is
# that body verbatim. Units and locations are numeric ids (no
# basemap in the harness, exactly like the pilot scenarios).
from eudkit import *

P1 = 0
P8 = 7
ZERGLING = 37       # unit ids
HYDRALISK = 38
LOC_SPAWN = 1       # location ids
LOC_BASE = 2

MAX_WAVE = 8
INTERMISSION = 120  # loops between waves
SPAWN_PERIOD = 6    # loops between streamed units
START_LIVES = 10
CLEAR_ORE = 150
BONUS_GAS = 75
BONUS_EVERY = 3     # waves per gas bonus

# Deaths table addresses for Player 8's units (see scenario 53).
DEATHS_ZERGLING = 0x58A364 + 4 * P8 + 48 * ZERGLING
DEATHS_HYDRALISK = 0x58A364 + 4 * P8 + 48 * HYDRALISK

state = EUDVariable(0)          # 0 = intermission, 1 = assault
wave = EUDVariable(1)           # current/next wave 1..8
inter_timer = EUDVariable(0)    # loops spent in intermission
spawn_timer = EUDVariable(0)    # loops since the last spawn
spawn_left = EUDVariable(0)     # units still to spawn
alive = EUDVariable(0)          # living enemies (exact, see header)
spawn_alt = EUDVariable(0)      # 0 = zergling next, 1 = hydralisk
lives = EUDVariable(START_LIVES)
victory = EUDVariable(0)
defeat = EUDVariable(0)
waves_cleared = EUDVariable(0)
bonus_counter = EUDVariable(0)  # clears since the last gas bonus
total_spawned = EUDVariable(0)
leaks = EUDVariable(0)
prev_zergling = EUDVariable(0)  # deaths counters at last loop
prev_hydralisk = EUDVariable(0)
cur_zergling = EUDVariable(0)   # scratch: current counters
cur_hydralisk = EUDVariable(0)
spawn_which = EUDVariable(0)    # scratch: alt snapshot per spawn


def run():
    global state, wave, inter_timer, spawn_timer, spawn_left, alive
    global spawn_alt, lives, victory, defeat, waves_cleared
    global bonus_counter, total_spawned, leaks
    global prev_zergling, prev_hydralisk, cur_zergling, cur_hydralisk
    global spawn_which
    if EUDIf()(victory == 0):
        if EUDIf()(defeat == 0):
            # --- deaths delta -> alive -------------------------
            cur_zergling << f_dwread_epd(EPD(DEATHS_ZERGLING))
            cur_hydralisk << f_dwread_epd(EPD(DEATHS_HYDRALISK))
            if EUDIf()(cur_zergling > prev_zergling):
                alive -= cur_zergling - prev_zergling
            EUDEndIf()
            if EUDIf()(cur_hydralisk > prev_hydralisk):
                alive -= cur_hydralisk - prev_hydralisk
            EUDEndIf()
            prev_zergling << cur_zergling
            prev_hydralisk << cur_hydralisk

            # --- leaks (assault only) --------------------------
            if EUDIf()(state == 1):
                if EUDIf()(Bring(P8, AtLeast, 1, ZERGLING, LOC_BASE)):
                    # Killed where it stands; its delta lands next
                    # loop, so `alive` is not adjusted here.
                    DoActions(KillUnitAt(1, ZERGLING, LOC_BASE, P8))
                    lives -= 1
                    leaks += 1
                EUDEndIf()
                if EUDIf()(Bring(P8, AtLeast, 1, HYDRALISK, LOC_BASE)):
                    DoActions(KillUnitAt(1, HYDRALISK, LOC_BASE, P8))
                    lives -= 1
                    leaks += 1
                EUDEndIf()
                if EUDIf()(lives == 0):
                    defeat << 1
                EUDEndIf()
            EUDEndIf()

            # --- intermission ----------------------------------
            if EUDIf()(state == 0):
                inter_timer += 1
                if EUDIf()(inter_timer >= INTERMISSION):
                    inter_timer << 0
                    spawn_left << 6 + wave * 3
                    spawn_timer << 0
                    state << 1
                    DoActions(MinimapPing(LOC_SPAWN))
                EUDEndIf()
            EUDEndIf()

            # --- assault ---------------------------------------
            if EUDIf()(state == 1):
                if EUDIf()(spawn_left >= 1):
                    spawn_timer += 1
                    if EUDIf()(spawn_timer >= SPAWN_PERIOD):
                        spawn_timer << 0
                        spawn_left -= 1
                        alive += 1
                        total_spawned += 1
                        # Snapshot parity BEFORE flipping it:
                        # sequential statements would otherwise
                        # fire both branches in one tick.
                        spawn_which << spawn_alt
                        if EUDIf()(spawn_which == 0):
                            DoActions(CreateUnit(
                                1, ZERGLING, LOC_SPAWN, P8))
                            spawn_alt << 1
                        EUDEndIf()
                        if EUDIf()(spawn_which == 1):
                            DoActions(CreateUnit(
                                1, HYDRALISK, LOC_SPAWN, P8))
                            spawn_alt << 0
                        EUDEndIf()
                    EUDEndIf()
                EUDEndIf()

                # Wave clear: nothing to spawn, nothing alive.
                if EUDIf()(spawn_left == 0):
                    if EUDIf()(alive == 0):
                        DoActions(SetResources(P1, Add, CLEAR_ORE, Ore))
                        waves_cleared += 1
                        bonus_counter += 1
                        if EUDIf()(bonus_counter >= BONUS_EVERY):
                            bonus_counter << 0
                            DoActions(SetResources(
                                P1, Add, BONUS_GAS, Gas))
                        EUDEndIf()
                        if EUDIf()(wave >= MAX_WAVE):
                            victory << 1
                        EUDEndIf()
                        if EUDIf()(wave < MAX_WAVE):
                            wave += 1
                            state << 0
                            inter_timer << 0
                        EUDEndIf()
                    EUDEndIf()
                EUDEndIf()
            EUDEndIf()
        EUDEndIf()
    EUDEndIf()
