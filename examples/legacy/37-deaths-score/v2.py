# 37 - Deaths-table kill ledger + score (modern eudkit)
#
# Expresses ideal intent:
# 1. Native Deaths table ledger and Score trigger actions.
# 2. Declarative one-time announcement via `with once():` replacing
#    manual boolean flag variables and multi-frame guard checks.
# 3. Clean local session state.
import eudkit


MARINE = 0
ZERGLING = 37
HYDRALISK = 38
ULTRALISK = 39

from eudkit import session

@session.fn(main_loop=True)
def game_loop() -> None:
    if eudkit.Deaths(eudkit.CurrentPlayer, eudkit.Exactly, 1, ZERGLING):
        eudkit.SetDeaths(eudkit.CurrentPlayer, eudkit.Add, 1, MARINE)
        eudkit.SetScore(eudkit.CurrentPlayer, eudkit.Add, 100, eudkit.Custom)

    if eudkit.Deaths(eudkit.CurrentPlayer, eudkit.Exactly, 1, HYDRALISK):
        eudkit.SetDeaths(eudkit.CurrentPlayer, eudkit.Add, 2, MARINE)
        eudkit.SetScore(eudkit.CurrentPlayer, eudkit.Add, 250, eudkit.Custom)

    if eudkit.Deaths(eudkit.CurrentPlayer, eudkit.Exactly, 1, ULTRALISK):
        eudkit.SetDeaths(eudkit.CurrentPlayer, eudkit.Add, 5, MARINE)
        eudkit.SetScore(eudkit.CurrentPlayer, eudkit.Add, 500, eudkit.Custom)
        eudkit.DisplayText("Ultralisk down! +500 score.")

    if eudkit.Deaths(eudkit.CurrentPlayer, eudkit.AtLeast, 5, MARINE):
        eudkit.SetResources(eudkit.CurrentPlayer, eudkit.Add, 200, eudkit.Ore)
        eudkit.SetDeaths(eudkit.CurrentPlayer, eudkit.SetTo, 0, MARINE)
        eudkit.DisplayText("Kill bonus paid: +200 ore.")

    if eudkit.Score(eudkit.CurrentPlayer, eudkit.Custom, eudkit.AtLeast, 1000):
        with eudkit.once():
            eudkit.DisplayText("Veteran! Score passed 1000.")
