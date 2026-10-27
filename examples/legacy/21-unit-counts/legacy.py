"""Scenario 21 (legacy) — existing vs completed unit counts.

Intent: every frame, answer four questions for Player 1 as deaths
flags: Marines existing >= 1 (208), Siege Tanks existing >= 3
(209), Marines completed >= 1 (210), Siege Tanks completed >= 3
(211).
Legacy idiom: "existing" via the engine's Command condition (the
textbook approach — but Command reads the engine's cached count
tables, which edac/142558 warns are unreliable for incomplete or
transforming units); "completed" has no Command form, so authors
drop to a raw Memory condition on the Completed Unit Counts Table.
"""

from eudkit import *  # noqa: F403

MARINE = 0  # Terran Marine unit id
SIEGE_TANK = 5  # Terran Siege Tank (Tank Mode) unit id
P1 = 0  # Player 1 index

# Unit-count tables (verified against the EUD Book):
#   existing  = 0x58A364 + 4 * Deaths(-8208) = 0x582324
#   completed = 0x58A364 + 4 * Deaths(-5472) = 0x584DE4
# Entry (player, unit) lives at base + 48 * unit + 4 * player.
EXISTING_BASE = 0x582324
COMPLETED_BASE = 0x584DE4


def counts_addr(base, player, unit):
    return base + 48 * unit + 4 * player


def run():
    # Existing counts: engine Command conditions (cached tables).
    if EUDIf()(Command(Player1, AtLeast, 1, MARINE)):
        DoActions([SetDeaths(Player1, SetTo, 1, 208)])
    EUDEndIf()
    if EUDIf()(Command(Player1, AtLeast, 3, SIEGE_TANK)):
        DoActions([SetDeaths(Player1, SetTo, 1, 209)])
    EUDEndIf()

    # Completed counts: raw Memory conditions on the counts table
    # (the Deaths(-5472) form, addressed directly).
    if EUDIf()(Memory(counts_addr(COMPLETED_BASE, P1, MARINE), AtLeast, 1)):
        DoActions([SetDeaths(Player1, SetTo, 1, 210)])
    EUDEndIf()
    if EUDIf()(Memory(counts_addr(COMPLETED_BASE, P1, SIEGE_TANK), AtLeast, 3)):
        DoActions([SetDeaths(Player1, SetTo, 1, 211)])
    EUDEndIf()
