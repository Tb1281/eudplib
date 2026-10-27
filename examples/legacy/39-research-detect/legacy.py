# 39 - research completion detection (legacy eudplib)
#
# Intent: watch the researched-tech state bytes for players 1-4; the
# first time a player's Stim Packs byte becomes non-zero, pay that
# player a one-time +100 gas bonus and announce it. A second watch does
# the same for Lockdown (tech id 1) with a +50 ore bonus.
#
# State layout (from legacy scdata TrgTech): SC tech state bytes live at
#   0x58CF44 + tech_id + player * 24     (one byte per tech per player)
# and Stim Packs is tech id 0.
#
# Counting form (pilot convention): run() builds the scenario's
# triggers once; benchmarks/pilot/count_legacy.py calls it inside a
# fresh PushTriggerScope and reports the GetTriggerCounter() delta.
# In production the intro is the onPluginStart hook and the watches
# are the beforeTriggerExec hook; run() folds both build passes into
# one, creating the identical triggers. Text is a Db string (no
# basemap in the harness, like the pilot scenarios). Every f_bread
# call site rides the import-time readtable engine (80T, built once
# per session and outside the delta) -- the amortization asymmetry
# the pilot RESULTS calls out.
from eudkit import *

RESEARCHED_BASE = 0x58CF44
STIM_PACKS = 0
LOCKDOWN = 1
BONUS_GAS = 100
LOCKDOWN_BONUS_ORE = 50

stim_rewarded = [EUDVariable() for _ in range(4)]
lockdown_rewarded = [EUDVariable() for _ in range(4)]


def run():
    for var in stim_rewarded + lockdown_rewarded:
        var << 0
    DoActions(DisplayText(Db("Research watch is active (Stim Packs / Lockdown).")))

    for player in range(4):
        state = f_bread(RESEARCHED_BASE + player * 24 + STIM_PACKS)
        if EUDIf()(state != 0):
            if EUDIf()(stim_rewarded[player] == 0):
                stim_rewarded[player] << 1
                DoActions([
                    SetResources(EncodePlayer(player), Add, BONUS_GAS, Gas),
                    DisplayText(Db("Stim Packs research complete! +100 gas.")),
                ])
            EUDEndIf()
        EUDEndIf()

    for player in range(4):
        state = f_bread(RESEARCHED_BASE + player * 24 + LOCKDOWN)
        if EUDIf()(state != 0):
            if EUDIf()(lockdown_rewarded[player] == 0):
                lockdown_rewarded[player] << 1
                DoActions([
                    SetResources(EncodePlayer(player), Add,
                                 LOCKDOWN_BONUS_ORE, Ore),
                    DisplayText(Db("Lockdown research complete! +50 ore.")),
                ])
            EUDEndIf()
        EUDEndIf()
