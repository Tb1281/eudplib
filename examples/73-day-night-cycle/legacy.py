# 73 - day / night cycle (legacy eudplib)
#
# Intent: the map alternates 600 loops of day and 600 loops of
# night. The phase drives three things:
#   spawn table: every 100 loops the enemy base produces its
#     wave -- by day 2 Zerglings, by night 1 Hydralisk (fewer,
#     stronger).
#   buff: the Zergling Claws weapon damage (weapons.dat field,
#     a word at 0x656EB0 + 2 * weapon id) is patched on every
#     transition: 5 by day, 8 by night -- nocturnal predators
#     hit harder.
#   mood: each transition is announced and counted; completed
#     day->night->day rounds increment the cycle counter.
#
# Counting form (pilot convention): run() builds the scenario's
# triggers once; benchmarks/pilot/count_legacy.py calls it inside
# a fresh PushTriggerScope and reports the GetTriggerCounter()
# delta. Units, locations and players are numeric ids (no
# basemap); text is a Db string, as in the pilot scenarios.
from eudkit import *

ZERGLING = 37
HYDRALISK = 38
CLAWS = 35                       # Zergling weapon id
WEAPON_DMG_BASE = 0x656EB0       # damage word per weapon id
ENEMY = 7                        # Player 8
ENEMY_BASE = 50
PHASE_LEN = 600                  # loops per phase
SPAWN_PERIOD = 100
CLAWS_DAY = 5
CLAWS_NIGHT = 8

phase_tick = EUDVariable()       # loops into the current phase
phase = EUDVariable()            # 0 day, 1 night
prev_phase = EUDVariable()
spawn_tick = EUDVariable()
day_spawns = EUDVariable()
night_spawns = EUDVariable()
cycles = EUDVariable()           # full day+night rounds completed
transitions = EUDVariable()


def on_transition():
    global transitions, cycles
    transitions += 1
    if EUDIf()(phase == 1):
        # Night falls: buff the claws, announce.
        f_wwrite(WEAPON_DMG_BASE + 2 * CLAWS, CLAWS_NIGHT)
        DoActions(DisplayText(Db("Night falls. The predators grow bold.")))
    if EUDElse()():
        # Dawn: restore the day table, count the round.
        f_wwrite(WEAPON_DMG_BASE + 2 * CLAWS, CLAWS_DAY)
        cycles += 1
        DoActions(DisplayText(Db("Dawn breaks. The swarm rests.")))
    EUDEndIf()


def run():
    global phase_tick, phase, prev_phase, spawn_tick
    global day_spawns, night_spawns
    phase_tick += 1
    spawn_tick += 1

    # Phase flip at the boundary.
    if EUDIf()(phase_tick >= PHASE_LEN):
        phase_tick << 0
        if EUDIf()(phase == 0):
            phase << 1
        if EUDElse()():
            phase << 0
        EUDEndIf()
    EUDEndIf()

    # Edge detection: fire the transition logic exactly once.
    if EUDIf()(phase != prev_phase):
        on_transition()
        prev_phase << phase
    EUDEndIf()

    # The phase spawn table.
    if EUDIf()(spawn_tick >= SPAWN_PERIOD):
        spawn_tick << 0
        if EUDIf()(phase == 0):
            DoActions(CreateUnit(2, ZERGLING, ENEMY_BASE,
                                 EncodePlayer(ENEMY)))
            day_spawns += 2
        if EUDElse()():
            DoActions(CreateUnit(1, HYDRALISK, ENEMY_BASE,
                                 EncodePlayer(ENEMY)))
            night_spawns += 1
        EUDEndIf()
    EUDEndIf()
