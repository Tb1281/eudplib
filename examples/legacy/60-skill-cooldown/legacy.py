# 60 - skill: mana, cooldown, area damage (legacy eudplib)
#
# Intent: a hero "Nova" skill for Player 1. The hero holds up to
# 100 mana. Mana regenerates +3 per loop while the skill is ready
# (cooldown 0) and +1 per loop while it cools down -- charging is
# part of the tactics: a full bar is required to cast. Casting
# (request switch SW 8) at full mana and zero cooldown unleashes
# the nova at the nova zone (location 5): up to 4 Zerglings and 2
# Hydralisks of Player 8 die, a minimap ping marks the blast, the
# bar empties, and a 240-loop cooldown starts. A cast request that
# cannot fire (bar not full, or still cooling down) is consumed
# and counted as a failed cast. The moment the skill becomes ready
# again (cooldown 0 AND mana full) a ready ping sounds at the hero
# (location 6), once per charge cycle.
#
# Counting form (pilot convention): run() builds the scenario's
# triggers once; benchmarks/pilot/count_legacy.py calls it inside a
# fresh PushTriggerScope and reports the GetTriggerCounter() delta.
# In production the body is the beforeTriggerExec hook; run() is
# that body verbatim. Units, locations and switches are numeric
# ids (no basemap in the harness, exactly like the pilot scenarios).
from eudkit import *

P8 = 7
ZERGLING = 37       # unit ids
HYDRALISK = 38
LOC_NOVA = 5        # location ids
LOC_HERO = 6

MANA_MAX = 100
REGEN_READY = 3     # mana per loop while the skill is ready
REGEN_COOLING = 1   # mana per loop while cooling down
NOVA_COOLDOWN = 240  # loops
NOVA_ZERGLINGS = 4  # nova kill quotas
NOVA_HYDRALISKS = 2
SW_NOVA = 8         # switch id

mana = EUDVariable(0)           # 0..100
cooldown = EUDVariable(0)       # loops until the skill is ready
ready_flag = EUDVariable(0)     # ready ping already sounded
casts = EUDVariable(0)          # lifetime casts
failed_casts = EUDVariable(0)    # requests that could not fire
total_mana_spent = EUDVariable(0)  # lifetime mana spent
castable = EUDVariable(0)       # scratch: cast check flag


def run():
    global mana, cooldown, ready_flag, casts, failed_casts
    global total_mana_spent, castable

    # Cooldown ticks down first; regen rates read the NEW value,
    # so the loop the cooldown expires already regens fast.
    if EUDIf()(cooldown >= 1):
        cooldown -= 1
    EUDEndIf()

    # Mana regeneration: fast while ready, slow while cooling.
    if EUDIf()(cooldown == 0):
        if EUDIf()(mana <= MANA_MAX - REGEN_READY):
            mana += REGEN_READY
        EUDEndIf()
        if EUDIf()(mana > MANA_MAX - REGEN_READY):
            if EUDIf()(mana < MANA_MAX):
                mana << MANA_MAX
            EUDEndIf()
        EUDEndIf()
    EUDEndIf()
    if EUDIf()(cooldown >= 1):
        if EUDIf()(mana < MANA_MAX):
            mana += REGEN_COOLING
        EUDEndIf()
    EUDEndIf()

    # Cast request. Legacy evaluates "castable" into a flag first
    # (compound and+not over two vars is a flag pattern here; the
    # ours side builds the compound Cond and its negation).
    if EUDIf()(Switch(SW_NOVA, Set)):
        DoActions(SetSwitch(SW_NOVA, Clear))
        castable << 0
        if EUDIf()(mana == MANA_MAX):
            if EUDIf()(cooldown == 0):
                castable << 1
            EUDEndIf()
        EUDEndIf()
        if EUDIf()(castable == 1):
            DoActions([
                KillUnitAt(NOVA_ZERGLINGS, ZERGLING, LOC_NOVA, P8),
                KillUnitAt(NOVA_HYDRALISKS, HYDRALISK, LOC_NOVA, P8),
                MinimapPing(LOC_NOVA),
            ])
            mana << 0
            cooldown << NOVA_COOLDOWN
            ready_flag << 0
            casts += 1
            total_mana_spent += MANA_MAX
        EUDEndIf()
        if EUDIf()(castable == 0):
            failed_casts += 1
        EUDEndIf()
    EUDEndIf()

    # Ready notification: once per charge cycle.
    if EUDIf()(cooldown == 0):
        if EUDIf()(mana == MANA_MAX):
            if EUDIf()(ready_flag == 0):
                DoActions(MinimapPing(LOC_HERO))
                ready_flag << 1
            EUDEndIf()
        EUDEndIf()
    EUDEndIf()
