# 53 - experience / level-up (legacy eudplib)
#
# Intent: an RPG hero progression system for Player 1. Enemy
# (Player 8) deaths are watched through the Deaths table: each
# Zergling kill is worth 5 XP, each Hydralisk 15 XP, each
# Ultralisk 40 XP. XP accumulates inside the current level; the
# amount needed grows per level (100 + 20 * (level - 1), levels
# 1..9, level 10 is the cap). On level-up: +150 ore and +25 gas,
# a minimap ping at the base, and bookkeeping (lifetime XP,
# level-ups, kills per type).
#
# Mechanism note: the level-up threshold is a per-level constant
# dispatched on the level var (EUDSwitch), NOT a computed var
# threshold -- the ours side cannot lower a var-vs-var branch
# comparison yet (known gap), so BOTH sides use the switch form
# to keep the pair intent-identical. It is also exact: at most one
# level-up can be pending per loop because the largest single-loop
# XP gain is smaller than the smallest threshold.
#
# Counting form (pilot convention): run() builds the scenario's
# triggers once; benchmarks/pilot/count_legacy.py calls it inside a
# fresh PushTriggerScope and reports the GetTriggerCounter() delta.
# In production the body is the beforeTriggerExec hook; run() is
# that body verbatim. Units are numeric ids (no basemap in the
# harness, exactly like the pilot scenarios).
from eudkit import *

P1 = 0
P8 = 7
LOC_BASE = 1        # location id

ZERGLING = 37       # unit ids
HYDRALISK = 38
ULTRALISK = 39
XP_ZERGLING = 5
XP_HYDRALISK = 15
XP_ULTRALISK = 40
MAX_LEVEL = 10
LEVEL_ORE = 150
LEVEL_GAS = 25

# Deaths table addresses for Player 8's units:
# 0x58A364 + 4*player + 48*unit, player 7.
DEATHS_ZERGLING = 0x58A364 + 4 * P8 + 48 * ZERGLING
DEATHS_HYDRALISK = 0x58A364 + 4 * P8 + 48 * HYDRALISK
DEATHS_ULTRALISK = 0x58A364 + 4 * P8 + 48 * ULTRALISK

level = EUDVariable(1)          # hero level 1..10
xp_in_level = EUDVariable(0)    # XP banked toward the next level
total_xp = EUDVariable(0)       # lifetime XP earned
level_ups = EUDVariable(0)      # lifetime level-ups
kills_zergling = EUDVariable(0)  # lifetime kills per type
kills_hydralisk = EUDVariable(0)
kills_ultralisk = EUDVariable(0)
prev_zergling = EUDVariable(0)  # deaths counters at last loop
prev_hydralisk = EUDVariable(0)
prev_ultralisk = EUDVariable(0)
cur_zergling = EUDVariable(0)   # scratch: current counters
cur_hydralisk = EUDVariable(0)
cur_ultralisk = EUDVariable(0)
gained = EUDVariable(0)         # scratch: XP gained this loop
delta = EUDVariable(0)          # scratch: per-type kill delta


def need_for(l):
    """XP needed to leave level l (constant per level)."""
    return 100 + 20 * (l - 1)


def run():
    global level, xp_in_level, total_xp, level_ups, gained, delta
    global kills_zergling, kills_hydralisk, kills_ultralisk
    global prev_zergling, prev_hydralisk, prev_ultralisk
    global cur_zergling, cur_hydralisk, cur_ultralisk

    # Read all three counters every loop; the deltas are exact
    # because Player 8 fields only these unit types in this system.
    cur_zergling << f_dwread_epd(EPD(DEATHS_ZERGLING))
    cur_hydralisk << f_dwread_epd(EPD(DEATHS_HYDRALISK))
    cur_ultralisk << f_dwread_epd(EPD(DEATHS_ULTRALISK))

    gained << 0
    if EUDIf()(cur_zergling > prev_zergling):
        delta << cur_zergling - prev_zergling
        gained += delta * XP_ZERGLING
        kills_zergling += delta
    EUDEndIf()
    if EUDIf()(cur_hydralisk > prev_hydralisk):
        delta << cur_hydralisk - prev_hydralisk
        gained += delta * XP_HYDRALISK
        kills_hydralisk += delta
    EUDEndIf()
    if EUDIf()(cur_ultralisk > prev_ultralisk):
        delta << cur_ultralisk - prev_ultralisk
        gained += delta * XP_ULTRALISK
        kills_ultralisk += delta
    EUDEndIf()

    prev_zergling << cur_zergling
    prev_hydralisk << cur_hydralisk
    prev_ultralisk << cur_ultralisk

    if EUDIf()(gained >= 1):
        xp_in_level += gained
        total_xp += gained
    EUDEndIf()

    # Level-up: one EUDSwitch case per level, threshold constant
    # inside each case (see the header's mechanism note).
    EUDSwitch(level)
    for l in range(1, MAX_LEVEL):
        if EUDSwitchCase()(l):
            if EUDIf()(xp_in_level >= need_for(l)):
                xp_in_level -= need_for(l)
                level += 1
                level_ups += 1
                DoActions([
                    SetResources(P1, Add, LEVEL_ORE, Ore),
                    SetResources(P1, Add, LEVEL_GAS, Gas),
                    MinimapPing(LOC_BASE),
                ])
            EUDEndIf()
    EUDEndSwitch()
