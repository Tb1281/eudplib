# 43 - per-player round bonus + personal message (legacy eudplib)
#
# Intent: every ~15 seconds (252 trigger loops at Fastest), run a bonus
# round over all 8 players: anyone fielding at least 3 Marines gets
# +25 ore, and a corps of 10+ Marines earns an extra +10 gas. The
# confirmation messages are PERSONAL: they must appear only on
# the screen of the player who earned it. In legacy EUD that is done by
# testing f_getuserplayerid() (the local human's id, different on every
# machine) on the display path only - the resource grant itself is a
# synchronized game-state change and is NOT locally guarded.
#
# Counting form (pilot convention): run() builds the scenario's
# triggers once; benchmarks/pilot/count_legacy.py calls it inside a
# fresh PushTriggerScope and reports the GetTriggerCounter() delta.
# In production the intro is the onPluginStart hook and the round loop
# is the beforeTriggerExec hook; run() folds both build passes into
# one, creating the identical triggers. Units are referenced by id and
# text is a Db string (no basemap in the harness, exactly like the
# pilot scenarios); a real map build resolves the same calls by name
# and creates the identical triggers.
from eudkit import *

MARINE = 0
MARINES_REQUIRED = 3
BONUS_ORE = 25
ELITE_MARINES = 10
ELITE_GAS = 10
ROUND_INTERVAL = 252    # ~15 s at Fastest

round_tick = EUDVariable()


@EUDFunc
def grant_player(player):
    if EUDIf()(Command(player, AtLeast, MARINES_REQUIRED, MARINE)):
        DoActions(SetResources(player, Add, BONUS_ORE, Ore))
        # Personal confirmation: only on this player's own machine.
        if EUDIf()(f_getuserplayerid() == player):
            DoActions(DisplayText(Db("Marine corps bonus paid: +25 ore.")))
        EUDEndIf()
        # Elite bonus: a big standing corps earns gas on top.
        if EUDIf()(Command(player, AtLeast, ELITE_MARINES, MARINE)):
            DoActions(SetResources(player, Add, ELITE_GAS, Gas))
            if EUDIf()(f_getuserplayerid() == player):
                DoActions(DisplayText(Db("Elite corps bonus paid: +10 gas.")))
            EUDEndIf()
        EUDEndIf()
    EUDEndIf()


def run():
    global round_tick
    round_tick << 0
    DoActions(DisplayText(Db("Marine corps bonus rounds are active.")))

    round_tick += 1
    if EUDIf()(round_tick >= ROUND_INTERVAL):
        round_tick << 0
        for player in range(8):
            grant_player(EncodePlayer(player))
    EUDEndIf()
