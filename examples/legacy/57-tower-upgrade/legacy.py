# 57 - tower upgrade (legacy eudplib)
#
# Intent: one defensive Siege Tank tower covering the tower zone
# (location 3). The tower's firepower is software-driven: every
# loop, if the cooldown has expired and enemies are in the zone,
# the tower "fires" (level + 1) shots -- each shot kills one enemy
# in the zone (Zerglings first, then Hydralisks) -- and the
# cooldown resets to 40 - 5*level loops. An upgrade request switch
# (SW 7) buys the next level for a per-level ore price
# (150 / 250 / 400), up to level 3; the request is consumed either
# way, and rejections (insufficient ore, already maxed) are
# counted. Kills are booked through Player 8's Deaths deltas (in
# this standalone scenario every Player 8 death is the tower's).
#
# Why a shot LOOP: action arguments are build-time values, so
# "kill (level+1) units" cannot be one KillUnitAt call with a
# variable count -- the idiom on both stacks is a counted loop of
# single kills, gated per shot by a fresh Bring check so the tower
# never fires into an empty zone.
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
LOC_TOWER = 3       # location id

MAX_LEVEL = 3
BASE_COOLDOWN = 40  # loops between volleys at level 0
CD_PER_LEVEL = 5    # cooldown reduction per level
UPGRADE_COSTS = [150, 250, 400]  # ore per next level 1/2/3
SW_UPGRADE = 7      # switch id

# Deaths table addresses for Player 8's units (see scenario 53).
DEATHS_ZERGLING = 0x58A364 + 4 * P8 + 48 * ZERGLING
DEATHS_HYDRALISK = 0x58A364 + 4 * P8 + 48 * HYDRALISK

tower_level = EUDVariable(0)    # 0..3
cooldown = EUDVariable(0)       # loops until the next volley
fired = EUDVariable(0)          # scratch: shots fired this volley
shots_fired = EUDVariable(0)    # lifetime shots
tower_kills = EUDVariable(0)    # lifetime kills (deaths delta)
upgrades = EUDVariable(0)       # successful upgrades
failed_buys = EUDVariable(0)    # rejected upgrade requests
ore_spent = EUDVariable(0)      # lifetime ore spent on upgrades
prev_zergling = EUDVariable(0)  # deaths counters at last loop
prev_hydralisk = EUDVariable(0)
cur_zergling = EUDVariable(0)   # scratch: current counters
cur_hydralisk = EUDVariable(0)
has_target = EUDVariable(0)     # scratch: enemy present in zone
req_level = EUDVariable(0)      # scratch: level when request arrived


def run():
    global tower_level, cooldown, fired, shots_fired, tower_kills
    global upgrades, failed_buys, ore_spent
    global prev_zergling, prev_hydralisk, cur_zergling, cur_hydralisk
    global has_target, req_level

    # --- kill bookkeeping (deaths deltas) ----------------------
    cur_zergling << f_dwread_epd(EPD(DEATHS_ZERGLING))
    cur_hydralisk << f_dwread_epd(EPD(DEATHS_HYDRALISK))
    if EUDIf()(cur_zergling > prev_zergling):
        tower_kills += cur_zergling - prev_zergling
    EUDEndIf()
    if EUDIf()(cur_hydralisk > prev_hydralisk):
        tower_kills += cur_hydralisk - prev_hydralisk
    EUDEndIf()
    prev_zergling << cur_zergling
    prev_hydralisk << cur_hydralisk

    # --- upgrade requests --------------------------------------
    if EUDIf()(Switch(SW_UPGRADE, Set)):
        DoActions(SetSwitch(SW_UPGRADE, Clear))
        # Snapshot the level at request time: the branches below
        # test the SNAPSHOT, because a successful buy mutates
        # tower_level and sequential branches would otherwise
        # chain-buy (and the maxed check would fire on the purchase
        # that REACHES max).
        req_level << tower_level
        # One branch per level: the price is a build-time constant
        # in each branch (the maxed case falls to rejection).
        for l in range(MAX_LEVEL):
            if EUDIf()(req_level == l):
                if EUDIf()(Accumulate(P1, AtLeast, UPGRADE_COSTS[l], Ore)):
                    DoActions(SetResources(
                        P1, Subtract, UPGRADE_COSTS[l], Ore))
                    tower_level += 1
                    upgrades += 1
                    ore_spent += UPGRADE_COSTS[l]
                    DoActions(MinimapPing(LOC_TOWER))
                EUDEndIf()
                if EUDIf()(Accumulate(P1, AtMost, UPGRADE_COSTS[l] - 1, Ore)):
                    failed_buys += 1
                EUDEndIf()
            EUDEndIf()
        if EUDIf()(req_level >= MAX_LEVEL):
            failed_buys += 1
        EUDEndIf()
    EUDEndIf()

    # --- tower fire --------------------------------------------
    if EUDIf()(cooldown >= 1):
        cooldown -= 1
    EUDEndIf()
    # Target presence as a flag (the ours side uses a compound
    # Bring-or-Bring condition for the same gate).
    has_target << 0
    if EUDIf()(Bring(P8, AtLeast, 1, ZERGLING, LOC_TOWER)):
        has_target << 1
    EUDEndIf()
    if EUDIf()(Bring(P8, AtLeast, 1, HYDRALISK, LOC_TOWER)):
        has_target << 1
    EUDEndIf()
    if EUDIf()(cooldown == 0):
        if EUDIf()(has_target == 1):
            # Fire (level + 1) shots; each shot re-checks its
            # target so a volley stops when the zone empties.
            fired << 0
            if EUDWhile()(fired <= tower_level):
                if EUDIf()(Bring(P8, AtLeast, 1, ZERGLING, LOC_TOWER)):
                    DoActions(KillUnitAt(1, ZERGLING, LOC_TOWER, P8))
                    fired += 1
                    shots_fired += 1
                EUDEndIf()
                if EUDIf()(Bring(P8, AtMost, 0, ZERGLING, LOC_TOWER)):
                    if EUDIf()(Bring(P8, AtLeast, 1, HYDRALISK, LOC_TOWER)):
                        DoActions(KillUnitAt(
                            1, HYDRALISK, LOC_TOWER, P8))
                        fired += 1
                        shots_fired += 1
                    EUDEndIf()
                    if EUDIf()(Bring(P8, AtMost, 0, HYDRALISK, LOC_TOWER)):
                        fired << 99  # zone empty: end the volley
                    EUDEndIf()
                EUDEndIf()
            EUDEndWhile()
            cooldown << BASE_COOLDOWN - tower_level * CD_PER_LEVEL
        EUDEndIf()
    EUDEndIf()
