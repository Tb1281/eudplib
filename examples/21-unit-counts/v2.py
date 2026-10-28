"""Scenario 21: Existing vs Completed Unit Counts — Modern eudkit (v2).

Ideal Intent:
Evaluate unit counts for Player 1 across both existing (under construction / morphed)
and completed states:
1. Terran Marines existing >= 1 -> flag 208 (`Command`).
2. Terran Siege Tanks existing >= 3 -> flag 209 (`Command`).
3. Terran Marines completed >= 1 -> flag 210 (direct query on Completed Table at `0x584DE4`).
4. Terran Siege Tanks completed >= 3 -> flag 211.
"""

import eudkit
from eudkit import (
    session,
    AtLeast,
    Command,
    Memory,
    Player1,
    SetDeaths,
    SetTo,
)


COMPLETED_BASE = 0x584DE4


def counts_addr(base: int, player: int, unit: int) -> int:
    return base + 48 * unit + 4 * player

@session.fn
def game_loop():
    # Existing unit queries (using native Command condition)
    if Command(Player1, AtLeast, 1, "Terran Marine"):
        SetDeaths(Player1, SetTo, 1, 208)

    if Command(Player1, AtLeast, 3, "Terran Siege Tank (Tank Mode)"):
        SetDeaths(Player1, SetTo, 1, 209)

    # Completed unit queries (direct memory table query)
    if Memory(counts_addr(COMPLETED_BASE, 0, 0), AtLeast, 1):
        SetDeaths(Player1, SetTo, 1, 210)

    if Memory(counts_addr(COMPLETED_BASE, 0, 5), AtLeast, 3):
        SetDeaths(Player1, SetTo, 1, 211)
