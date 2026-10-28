"""Scenario 26: Cooldown Skill System — Modern eudkit (v2).

Ideal Intent:
Player 1 casts "Psionic Burst" by moving a Ghost into the cast zone (location 3):
1. Skill cooldown is 300 trigger cycles.
2. If Ghost enters while skill is ready (`ready == 1`):
   - Kill 2 Zerglings for Player 8.
   - Credit kills (`kills_credited += 2`) and record successful cast (`casts += 1`).
   - Trigger cooldown timer using reactive primitive `with after(299):`.
3. If Ghost enters while skill is cooling down (`cooling == 1`):
   - Ignore cast and increment `wasted` attempts counter.
"""

import eudkit
from eudkit import (
    session,
    AtLeast,
    Bring,
    Player1,
    Player8,
    SetDeaths,
    Subtract,
    after,
)


COOLDOWN_LOOPS = 300
LOC_CAST_ZONE = 3

ready = session.var(1, name="ready")
cooling = session.var(0, name="cooling")
casts = session.var(0, name="casts")
wasted = session.var(0, name="wasted")
kills_credited = session.var(0, name="kills_credited")

@session.fn
def game_loop():
    # Reactive delay primitive: re-arms ready flag exactly 300 ticks after cast
    if cooling == 1:
        with after(COOLDOWN_LOOPS - 1):
            ready = 1
            cooling = 0

    # Cast gate detection
    if Bring(Player1, AtLeast, 1, "Terran Ghost", LOC_CAST_ZONE):
        if cooling == 1:
            wasted += 1
        if ready == 1:
            SetDeaths(Player8, Subtract, 2, "Zerg Zergling")
            kills_credited += 2
            casts += 1
            ready = 0
            cooling = 1
