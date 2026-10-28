# 59 - hero revive (legacy eudplib)
#
# Intent: Player 1's hero is a single Marine. Death is detected
# when the hero is alive in our state machine but the player
# commands zero Marines. On death: a death counter increments, a
# minimap ping marks the base (location 1), and a revive countdown
# starts at 180 + 30 * (deaths so far) loops -- each death makes
# the next wait longer. While dead, dead-time is accumulated for
# stats. When the countdown expires, reviving costs 100 ore: if
# the player can pay, the fee is charged, the hero is recreated at
# base, and the state flips back to alive. If the player cannot
# pay, a waiting flag stays set (the UI can nag) and the revive
# retries every loop until affordable.
#
# Announcements use plain DisplayText of map strings -- deliberately
# NOT DBString: the text is constant, and constant text is cheapest
# as a map string on either stack. The strings are Db payloads
# here; a real map build references STR entries by name.
#
# Counting form (pilot convention): run() builds the scenario's
# triggers once; benchmarks/pilot/count_legacy.py calls it inside a
# fresh PushTriggerScope and reports the GetTriggerCounter() delta.
# In production the body is the beforeTriggerExec hook; run() is
# that body verbatim. Units and locations are numeric ids (no
# basemap in the harness, exactly like the pilot scenarios).
from eudkit import *

P1 = 0
MARINE = 0          # unit id
LOC_BASE = 1        # location id

MSG_FALLEN = Db("Hero has fallen")      # announcement payloads
MSG_RETURNED = Db("Hero has returned")

BASE_WAIT = 180     # loops until the first revive is possible
WAIT_PER_DEATH = 30  # extra wait per previous death
REVIVE_COST = 100   # ore

hero_state = EUDVariable(0)     # 0 = alive, 1 = dead
countdown = EUDVariable(0)      # loops left until revive window
waiting = EUDVariable(0)        # 1 while blocked on the fee
deaths = EUDVariable(0)         # lifetime hero deaths
revives = EUDVariable(0)        # lifetime revives
total_dead_loops = EUDVariable(0)  # lifetime loops spent dead
total_fees = EUDVariable(0)     # lifetime ore paid in fees


def run():
    global hero_state, countdown, waiting, deaths, revives
    global total_dead_loops, total_fees

    # Death detection: alive in our state, zero Marines commanded.
    if EUDIf()(hero_state == 0):
        if EUDIf()(Command(P1, Exactly, 0, MARINE)):
            hero_state << 1
            deaths += 1
            # The wait grows with the death count: 180 + 30*deaths,
            # computed AFTER this death is counted (first = 210).
            countdown << BASE_WAIT + deaths * WAIT_PER_DEATH
            DoActions([
                MinimapPing(LOC_BASE),
                DisplayText(MSG_FALLEN),
            ])
        EUDEndIf()
    EUDEndIf()

    # Dead: tick the countdown and the stats.
    if EUDIf()(hero_state == 1):
        total_dead_loops += 1
        if EUDIf()(countdown >= 1):
            countdown -= 1
        EUDEndIf()

        # Revive window: pay the fee and recreate the hero.
        if EUDIf()(countdown == 0):
            if EUDIf()(Accumulate(P1, AtLeast, REVIVE_COST, Ore)):
                DoActions([
                    SetResources(P1, Subtract, REVIVE_COST, Ore),
                    CreateUnit(1, MARINE, LOC_BASE, P1),
                    MinimapPing(LOC_BASE),
                    DisplayText(MSG_RETURNED),
                ])
                total_fees += REVIVE_COST
                hero_state << 0
                waiting << 0
                revives += 1
            EUDEndIf()
            # Cannot pay yet: raise the waiting flag and retry
            # next loop (the countdown stays at 0).
            if EUDIf()(Accumulate(P1, AtMost, REVIVE_COST - 1, Ore)):
                waiting << 1
            EUDEndIf()
        EUDEndIf()
    EUDEndIf()
