"""LEGACY turret aiming (benchmark 31).

Same intent as ours.js: a fixed turret at pixel (1280, 1280)
watches a target whose coordinates an external tracker writes
into target_x / target_y every loop. Each loop the turret
computes the distance to the target -- legacy uses the exact
f_sqrt EUDFunc on dx*dx + dy*dy -- and, when the target is within
range 384 and the 24-loop fire cooldown has elapsed, it kills one
of Player 8's Zerglings at the aim zone (location 6), which stands
in for the projectile hit. Bookkeeping: shots, out-of-range loops,
longest distance observed.
"""

from eudkit import *

TURRET_X = 1280
TURRET_Y = 1280
RANGE = 384
FIRE_COOLDOWN = 24       # loops between shots
LOC_AIM_ZONE = 6         # location id (numeric: no basemap lookup)
UNIT_ZERGLING = 37       # unit id (numeric form, as in the pilot)
MAX_TRACK_DIST = 2000    # distances beyond this are not recorded

target_x = EUDVariable()   # written by the external tracker
target_y = EUDVariable()   # written by the external tracker
dx = EUDVariable()         # |target_x - TURRET_X| (scratch)
dy = EUDVariable()         # |target_y - TURRET_Y| (scratch)
dist = EUDVariable()       # exact distance to the target
cooldown = EUDVariable()   # loops until the turret may fire again
shots = EUDVariable()
out_of_range = EUDVariable()  # loops with a target beyond range
longest = EUDVariable()       # longest distance observed


def run():
    global target_x, target_y, dx, dy, dist, cooldown, shots
    global out_of_range, longest
    # Absolute deltas (EUD variables are unsigned).
    if EUDIf()(target_x >= TURRET_X):
        dx << target_x - TURRET_X
    if EUDElse()():
        dx << TURRET_X - target_x
    EUDEndIf()
    if EUDIf()(target_y >= TURRET_Y):
        dy << target_y - TURRET_Y
    if EUDElse()():
        dy << TURRET_Y - target_y
    EUDEndIf()

    # Exact distance via the library square root (an EUDFunc call:
    # the call site is small, the body is shared library code).
    dist << f_sqrt(dx * dx + dy * dy)

    # Longest-observed tracking (bounded, so map-edge reads of the
    # tracker do not poison the statistic).
    if EUDIf()([dist > longest, dist <= MAX_TRACK_DIST]):
        longest << dist
    EUDEndIf()

    if EUDIf()(cooldown >= 1):
        cooldown -= 1
    EUDEndIf()

    # Fire decision.
    if EUDIf()(dist <= RANGE):
        if EUDIf()(cooldown == 0):
            DoActions(KillUnitAt(1, UNIT_ZERGLING, LOC_AIM_ZONE, Player8))
            shots += 1
            cooldown << FIRE_COOLDOWN
        EUDEndIf()
    if EUDElse()():
        out_of_range += 1
    EUDEndIf()
