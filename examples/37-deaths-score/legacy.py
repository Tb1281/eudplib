# 37 - Deaths-table kill ledger + score (legacy eudplib)
#
# Intent: keep a kill ledger in the Deaths table itself (the Marine
# death slot of the current player doubles as a counter - the classic
# eps trick), and pay score for kills:
#   Zergling kill  -> ledger +1, score +100
#   Hydralisk kill -> ledger +2, score +250
#   Ultralisk kill -> ledger +5, score +500
# When the ledger reaches 5, pay a 200-ore bonus and reset it to 0.
# At 1000 score, announce "Veteran!" once.
#
# Counting form (pilot convention): run() builds the scenario's
# triggers once; benchmarks/pilot/count_legacy.py calls it inside a
# fresh PushTriggerScope and reports the GetTriggerCounter() delta.
# In production this body is the beforeTriggerExec hook. Units are
# referenced by id and text is a Db string (no basemap in the harness,
# exactly like the pilot scenarios); a real map build resolves the
# same calls by name/literal and creates the identical triggers.
from eudkit import *

MARINE = 0
ZERGLING = 37
HYDRALISK = 38
ULTRALISK = 39

veteran_announced = EUDVariable()


def run():
    if EUDIf()(Deaths(CurrentPlayer, Exactly, 1, ZERGLING)):
        DoActions([
            SetDeaths(CurrentPlayer, Add, 1, MARINE),
            SetScore(CurrentPlayer, Add, 100, Custom),
        ])
    EUDEndIf()

    if EUDIf()(Deaths(CurrentPlayer, Exactly, 1, HYDRALISK)):
        DoActions([
            SetDeaths(CurrentPlayer, Add, 2, MARINE),
            SetScore(CurrentPlayer, Add, 250, Custom),
        ])
    EUDEndIf()

    if EUDIf()(Deaths(CurrentPlayer, Exactly, 1, ULTRALISK)):
        DoActions([
            SetDeaths(CurrentPlayer, Add, 5, MARINE),
            SetScore(CurrentPlayer, Add, 500, Custom),
            DisplayText(Db("Ultralisk down! +500 score.")),
        ])
    EUDEndIf()

    if EUDIf()(Deaths(CurrentPlayer, AtLeast, 5, MARINE)):
        DoActions([
            SetResources(CurrentPlayer, Add, 200, Ore),
            SetDeaths(CurrentPlayer, SetTo, 0, MARINE),
            DisplayText(Db("Kill bonus paid: +200 ore.")),
        ])
    EUDEndIf()

    if EUDIf()(Score(CurrentPlayer, Custom, AtLeast, 1000)):
        if EUDIf()(veteran_announced == 0):
            veteran_announced << 1
            DoActions(DisplayText(Db("Veteran! Score passed 1000.")))
        EUDEndIf()
    EUDEndIf()
