# 38 - elapsed-time staged events (legacy eudplib)
#
# Intent: a five-stage timeline driven by the game clock.
#   10 s: ready warning.
#   30 s: announce wave 1, spawn 4 Marines for P2 at "Spawn".
#   90 s: announce wave 2, spawn 8 Zerglings split over two spawns.
#  180 s: boss warning, spawn 1 Ultralisk + 4 Hydralisk escorts.
#  240 s: final wave, 12 Hydralisks split over two spawns.
# Each stage must fire exactly once: legacy has no once-primitive, so a
# stage variable guards every block forever - the guard triggers keep
# being evaluated on every frame for the rest of the game.
#
# Counting form (pilot convention): run() builds the scenario's
# triggers once; benchmarks/pilot/count_legacy.py calls it inside a
# fresh PushTriggerScope and reports the GetTriggerCounter() delta.
# In production this body is the beforeTriggerExec hook. Units and
# locations are referenced by id and text is a Db string (no basemap
# in the harness, exactly like the pilot scenarios); a real map build
# resolves the same calls by name and creates the identical triggers.
from eudkit import *

MARINE = 0
ZERGLING = 37
HYDRALISK = 38
ULTRALISK = 39

SPAWN = 1     # location id of "Spawn" in the target map
SPAWN_B = 2   # location id of "Spawn B" in the target map

stage = EUDVariable()


def run():
    if EUDIf()(ElapsedTime(AtLeast, 10)):
        if EUDIf()(stage == 0):
            stage << 1
            DoActions(DisplayText(Db("Get ready - first wave in 20 seconds.")))
        EUDEndIf()
    EUDEndIf()

    if EUDIf()(ElapsedTime(AtLeast, 30)):
        if EUDIf()(stage == 1):
            stage << 2
            DoActions([
                DisplayText(Db("Wave 1 incoming!")),
                CreateUnit(4, MARINE, SPAWN, P2),
            ])
        EUDEndIf()
    EUDEndIf()

    if EUDIf()(ElapsedTime(AtLeast, 90)):
        if EUDIf()(stage == 2):
            stage << 3
            DoActions([
                DisplayText(Db("Wave 2 incoming!")),
                CreateUnit(4, ZERGLING, SPAWN, P2),
                CreateUnit(4, ZERGLING, SPAWN_B, P2),
            ])
        EUDEndIf()
    EUDEndIf()

    if EUDIf()(ElapsedTime(AtLeast, 180)):
        if EUDIf()(stage == 3):
            stage << 4
            DoActions([
                DisplayText(Db("WARNING: something huge approaches...")),
                CreateUnit(1, ULTRALISK, SPAWN, P2),
                CreateUnit(4, HYDRALISK, SPAWN_B, P2),
            ])
        EUDEndIf()
    EUDEndIf()

    if EUDIf()(ElapsedTime(AtLeast, 240)):
        if EUDIf()(stage == 4):
            stage << 5
            DoActions([
                DisplayText(Db("Final wave! Hold the line!")),
                CreateUnit(6, HYDRALISK, SPAWN, P2),
                CreateUnit(6, HYDRALISK, SPAWN_B, P2),
            ])
        EUDEndIf()
    EUDEndIf()
