"""LEGACY cooldown skill (benchmark 26).

Same intent as ours.js: Player 1 casts "psionic burst" by bringing
a Ghost to the cast zone (location 3). The burst kills up to 2 of
Player 8's Zerglings in the zone. After a cast the skill cools down
for 300 trigger loops; cast attempts during the cooldown are
ignored. The legacy idiom is a hand-rolled countdown variable,
decremented once per loop before the cast gate is checked.
Bookkeeping: casts landed, bursts wasted (attempts on cooldown),
total kills credited.
"""

from eudkit import *

COOLDOWN_LOOPS = 300     # loops between casts
LOC_CAST_ZONE = 3        # location id (numeric: no basemap lookup)
UNIT_GHOST = 1           # unit ids (numeric form, as in the pilot)
UNIT_ZERGLING = 37
BURST_KILLS = 2          # zerglings removed per cast

cooldown = EUDVariable()     # loops of cooldown remaining (0 = ready)
casts = EUDVariable()        # bursts successfully cast
wasted = EUDVariable()       # cast attempts ignored (on cooldown)
kills_credited = EUDVariable()  # zerglings removed by bursts


def run():
    global cooldown, casts, wasted, kills_credited
    # 1) Tick the countdown first, so a cast on loop L re-opens the
    # gate exactly on loop L + COOLDOWN_LOOPS.
    if EUDIf()(cooldown >= 1):
        cooldown -= 1
    EUDEndIf()

    # 2) Cast gate: ghost in the zone.
    if EUDIf()(Bring(Player1, AtLeast, 1, UNIT_GHOST, LOC_CAST_ZONE)):
        if EUDIf()(cooldown == 0):
            # Fire the burst and start the cooldown.
            DoActions(KillUnitAt(BURST_KILLS, UNIT_ZERGLING,
                                 LOC_CAST_ZONE, Player8))
            kills_credited += BURST_KILLS
            casts += 1
            cooldown << COOLDOWN_LOOPS
        if EUDElse()():
            # Attempt while cooling down: counted, no effect.
            wasted += 1
        EUDEndIf()
    EUDEndIf()
