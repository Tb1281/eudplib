"""Scenario 20: Bring Condition Area Count — Modern eudkit (v2).

Ideal Intent:
Evaluate player unit presence inside a target location using native engine queries:
1. Test if Player 1 has at least 1 Terran Marine in location 1 -> flag 205.
2. Test if Player 1 has at least 3 Terran Marines in location 1 -> flag 206.
3. Test if Player 1 has at least 5 Terran Marines in location 1 -> flag 207.
"""

import eudkit
from eudkit import (
    session,
    AtLeast,
    Bring,
    Player1,
    SetDeaths,
    SetTo,
)

@session.fn
def game_loop():
    # High-level typed Bring conditions evaluated natively by StarCraft's trigger engine
    if Bring(Player1, AtLeast, 1, "Terran Marine", 1):
        SetDeaths(Player1, SetTo, 1, 205)

    if Bring(Player1, AtLeast, 3, "Terran Marine", 1):
        SetDeaths(Player1, SetTo, 1, 206)

    if Bring(Player1, AtLeast, 5, "Terran Marine", 1):
        SetDeaths(Player1, SetTo, 1, 207)
