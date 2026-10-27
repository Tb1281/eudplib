# 42 - MoveLocation follow pattern (legacy eudplib)
#
# Intent: a "Track Zone" location recenters on the current player's
# first Marine every frame; when 3+ Zerglings are inside it, that is
# first contact: announce once and start paying +10 score per ~5 s
# while contact holds. A second, static "Base Zone" raises one-time
# minimap pings if a Hydralisk slips in, or if 5+ Zerglings overrun it.
#
# MoveLocation is also the cache-safe way to do this: the engine caches
# Bring results within a frame, and unit-moving actions do not all
# invalidate the cache - but MoveLocation does, so the Bring below sees
# this frame's recentered zone (edac/125639 pattern).
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

TRACK_ZONE = 1   # location id of "Track Zone" in the target map
BASE_ZONE = 2    # location id of "Base Zone" in the target map
ANYWHERE = 0     # location id 0 is the engine's Anywhere sentinel

contact_made = EUDVariable()
base_alerted = EUDVariable()
base_overrun = EUDVariable()
score_cd = EUDVariable()


def run():
    global score_cd
    # Recenter first: every check below uses this frame's zone.
    DoActions(MoveLocation(TRACK_ZONE, MARINE, CurrentPlayer, ANYWHERE))

    if EUDIf()(score_cd >= 1):
        score_cd -= 1
    EUDEndIf()

    if EUDIf()(Bring(CurrentPlayer, AtLeast, 3, ZERGLING, TRACK_ZONE)):
        if EUDIf()(contact_made == 0):
            contact_made << 1
            DoActions(DisplayText(Db("Contact! Zerglings on our Marine.")))
        EUDEndIf()
        if EUDIf()(score_cd == 0):
            score_cd << 84
            DoActions(SetScore(CurrentPlayer, Add, 10, Custom))
        EUDEndIf()
    EUDEndIf()

    if EUDIf()(Bring(P2, AtLeast, 1, HYDRALISK, BASE_ZONE)):
        if EUDIf()(base_alerted == 0):
            base_alerted << 1
            DoActions([
                MinimapPing(BASE_ZONE),
                DisplayText(Db("A Hydralisk slipped into the Base Zone!")),
            ])
        EUDEndIf()
    EUDEndIf()

    if EUDIf()(Bring(P2, AtLeast, 5, ZERGLING, BASE_ZONE)):
        if EUDIf()(base_overrun == 0):
            base_overrun << 1
            DoActions([
                MinimapPing(BASE_ZONE),
                DisplayText(Db("The base is being overrun!")),
            ])
        EUDEndIf()
    EUDEndIf()
