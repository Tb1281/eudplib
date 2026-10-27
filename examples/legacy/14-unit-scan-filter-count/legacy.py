"""Scenario 14 (legacy) — full unit-table scan + type filter/count.

Intent: every frame, walk all 1700 CUnit slots; count live units of
each kind we care about:
  - Player 1's Terran Marines (unit id 0) with hp >= 256 ("veterans")
  - Player 1's Terran Marines below 256 hp ("wounded")
  - every live unit on the map, any owner
Legacy idiom: EUDLoopCUnit() + CUnit member comparisons.
"""

from eudkit import *  # noqa: F403

MARINE = 0  # Terran Marine unit id
NONE_TYPE = 228  # "None" unit id: empty/dead slot

total_live = EUDVariable()
p1_veterans = EUDVariable()
p1_wounded = EUDVariable()
p1_marines = EUDVariable()


def count_all_units():
    """One full sweep over the 1700-slot unit table."""
    global total_live, p1_veterans, p1_wounded, p1_marines
    total_live << 0
    p1_veterans << 0
    p1_wounded << 0
    p1_marines << 0

    for cu in EUDLoopCUnit():
        # Dead-skip: an empty slot still carries unitType "None".
        if EUDIf()(cu.unitType == NONE_TYPE):
            EUDContinue()
        EUDEndIf()
        total_live += 1

        # Owner filter: only Player 1 (index 0) units continue below.
        if EUDIf()(cu.owner != 0):
            EUDContinue()
        EUDEndIf()

        # Type filter: Marines only.
        if EUDIf()(cu.unitType != MARINE):
            EUDContinue()
        EUDEndIf()
        p1_marines += 1

        # HP split at the 256 boundary.
        if EUDIf()(cu.hp >= 256):
            p1_veterans += 1
        if EUDElse()():
            p1_wounded += 1
        EUDEndIf()


def run():
    count_all_units()
    # Scoreboard flags gated on the counters we just computed
    # (string-free reporting: deaths counters on unused unit slots
    # 200/201, per the pilot harness convention — see note.md).
    if EUDIf()(p1_veterans >= 10):
        DoActions([SetDeaths(Player1, SetTo, 1, 200)])
    EUDEndIf()
    if EUDIf()(p1_wounded >= 1):
        DoActions([SetDeaths(Player1, SetTo, 1, 201)])
    EUDEndIf()
