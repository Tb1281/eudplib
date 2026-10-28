"""Scenario 20 (legacy) — count Player 1 Marines inside a location.

Intent: every frame, answer three threshold questions about P1
Marines inside location 1: at least 1 / at least 3 / at least 5 —
reported as deaths flags 205/206/207.
Legacy idiom: the engine's own Bring condition (one condition per
question). Known quirk (edac/125639): Bring results are CACHED
within a frame and CreateUnit/GiveUnits/KillUnit/RemoveUnit do NOT
invalidate the cache, so a map that creates units must run the
community workaround — a dummy RemoveUnitAt in the location, which
does invalidate — before trusting Bring.
"""

from eudkit import *  # noqa: F403

MARINE = 0  # Terran Marine unit id
NONE_TYPE = 228  # "None" unit id: the dummy for cache invalidation
SCAN_LOCATION = 1  # map location index (id form: no name lookup)


def run():
    # 1) Cache invalidation workaround: removing a (nonexistent)
    #    "None" unit in the location forces the engine to recompute
    #    its Bring tables before the checks below.
    DoActions([RemoveUnitAt(1, NONE_TYPE, SCAN_LOCATION, AllPlayers)])

    # 2) The threshold ladder: one engine condition per question.
    if EUDIf()(Bring(Player1, AtLeast, 1, MARINE, SCAN_LOCATION)):
        DoActions([SetDeaths(Player1, SetTo, 1, 205)])
    EUDEndIf()
    if EUDIf()(Bring(Player1, AtLeast, 3, MARINE, SCAN_LOCATION)):
        DoActions([SetDeaths(Player1, SetTo, 1, 206)])
    EUDEndIf()
    if EUDIf()(Bring(Player1, AtLeast, 5, MARINE, SCAN_LOCATION)):
        DoActions([SetDeaths(Player1, SetTo, 1, 207)])
    EUDEndIf()
