# 38 - elapsed-time staged events (modern eudkit)
#
# Expresses ideal intent:
# 1. Native `ElapsedTime` condition cleanly scheduled with declarative `with once():` blocks.
# 2. Eliminates the legacy `stage` variable and its redundant per-tick state checks.
# 3. Clean local session state with zero VProcs.
import eudkit


MARINE = 0
ZERGLING = 37
HYDRALISK = 38
ULTRALISK = 39

SPAWN = 1
SPAWN_B = 2

from eudkit import session

@session.fn(main_loop=True)
def game_loop() -> None:
    if eudkit.ElapsedTime(eudkit.AtLeast, 10):
        with eudkit.once():
            eudkit.DisplayText("Get ready - first wave in 20 seconds.")

    if eudkit.ElapsedTime(eudkit.AtLeast, 30):
        with eudkit.once():
            eudkit.DisplayText("Wave 1 incoming!")
            eudkit.CreateUnit(4, MARINE, SPAWN, eudkit.P2)

    if eudkit.ElapsedTime(eudkit.AtLeast, 90):
        with eudkit.once():
            eudkit.DisplayText("Wave 2 incoming!")
            eudkit.CreateUnit(4, ZERGLING, SPAWN, eudkit.P2)
            eudkit.CreateUnit(4, ZERGLING, SPAWN_B, eudkit.P2)

    if eudkit.ElapsedTime(eudkit.AtLeast, 180):
        with eudkit.once():
            eudkit.DisplayText("WARNING: something huge approaches...")
            eudkit.CreateUnit(1, ULTRALISK, SPAWN, eudkit.P2)
            eudkit.CreateUnit(4, HYDRALISK, SPAWN_B, eudkit.P2)

    if eudkit.ElapsedTime(eudkit.AtLeast, 240):
        with eudkit.once():
            eudkit.DisplayText("Final wave! Hold the line!")
            eudkit.CreateUnit(6, HYDRALISK, SPAWN, eudkit.P2)
            eudkit.CreateUnit(6, HYDRALISK, SPAWN_B, eudkit.P2)
