# 41 - minimap ping + warning with cooldown (legacy eudplib)
#
# Intent: guard three locations. When the enemy (P2) brings a unit into
# "Watch Zone" (any Marine), "Gate Zone" (any Hydralisk) or "Rear Zone"
# (any Zergling), ping that location on the minimap and print a warning
# - but at most once per ~10 seconds per zone, so a standing intruder
# does not spam the ping. Each zone gets its own cooldown counter,
# decremented every frame.
#
# Counting form (pilot convention): run() builds the scenario's
# triggers once; benchmarks/pilot/count_legacy.py calls it inside a
# fresh PushTriggerScope and reports the GetTriggerCounter() delta.
# In production this body is the beforeTriggerExec hook. Units and
# locations are referenced by id and text is a Db string (no basemap
# in the harness, exactly like the pilot scenarios); a real map build
# resolves the same calls by name and creates the identical triggers.
from eudkit import *

COOLDOWN_FRAMES = 168   # ~10 s at Fastest (16.7 trigger loops / s)
ENEMY = P2

MARINE = 0
HYDRALISK = 38
ZERGLING = 37

WATCH_ZONE = 1   # location id of "Watch Zone" in the target map
GATE_ZONE = 2    # location id of "Gate Zone" in the target map
REAR_ZONE = 3    # location id of "Rear Zone" in the target map

watch_cd = EUDVariable()
gate_cd = EUDVariable()
rear_cd = EUDVariable()


def run():
    global watch_cd, gate_cd, rear_cd
    if EUDIf()(watch_cd >= 1):
        watch_cd -= 1
    EUDEndIf()
    if EUDIf()(gate_cd >= 1):
        gate_cd -= 1
    EUDEndIf()
    if EUDIf()(rear_cd >= 1):
        rear_cd -= 1
    EUDEndIf()

    if EUDIf()(Bring(ENEMY, AtLeast, 1, MARINE, WATCH_ZONE)):
        if EUDIf()(watch_cd == 0):
            watch_cd << COOLDOWN_FRAMES
            DoActions([
                MinimapPing(WATCH_ZONE),
                DisplayText(Db("Warning: enemy Marine in the Watch Zone!")),
            ])
        EUDEndIf()
    EUDEndIf()

    if EUDIf()(Bring(ENEMY, AtLeast, 1, HYDRALISK, GATE_ZONE)):
        if EUDIf()(gate_cd == 0):
            gate_cd << COOLDOWN_FRAMES
            DoActions([
                MinimapPing(GATE_ZONE),
                DisplayText(Db("Warning: enemy Hydralisk at the Gate Zone!")),
            ])
        EUDEndIf()
    EUDEndIf()

    if EUDIf()(Bring(ENEMY, AtLeast, 1, ZERGLING, REAR_ZONE)):
        if EUDIf()(rear_cd == 0):
            rear_cd << COOLDOWN_FRAMES
            DoActions([
                MinimapPing(REAR_ZONE),
                DisplayText(Db("Warning: enemy Zergling in the Rear Zone!")),
            ])
        EUDEndIf()
    EUDEndIf()
