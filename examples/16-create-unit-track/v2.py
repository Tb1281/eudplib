"""Scenario 16: Unit Creation and Handle Tracking — Modern eudkit (v2).

Ideal Intent:
Every 24 trigger cycles (1 second at Fastest speed):
1. Capture the engine's "next empty unit" pointer from `0x628438` before creation.
2. Spawn 1 Terran Marine for Player 1.
3. Immediately buff the spawned unit's HP to 512.
4. Record its EPD handle and track spawn counts.
5. Set milestone death flag when `spawn_count >= 10`.
"""

import eudkit
from eudkit import (
    session,
    CreateUnit,
    EPD,
    Player1,
    SetDeaths,
    SetTo,
    dwread_epd,
    dwwrite_epd,
    epdread_epd,
    every,
)


NEXT_UNIT_EPD = EPD(0x628438)

spawn_count = session.var(0, name="spawn_count")
last_spawn_epd = session.var(0, name="last_spawn_epd")
last_spawn_hp = session.var(0, name="last_spawn_hp")
next_epd = session.var(0, name="next_epd")

@session.fn
def game_loop():
    # Reactive periodic rule: fires every 24 ticks without manual counter arithmetic
    with every(24):
        # 1. Capture the next unit EPD handle before spawning
        next_epd = epdread_epd(NEXT_UNIT_EPD)

        # 2. Spawn unit
        CreateUnit(1, "Terran Marine", "Anywhere", Player1)

        # 3. Apply custom stats to the newly created entity handle
        dwwrite_epd(next_epd + 2, 512)
        last_spawn_epd = next_epd
        spawn_count += 1
        last_spawn_hp = dwread_epd(next_epd + 2)

    # Milestone achievement
    if spawn_count >= 10:
        SetDeaths(Player1, SetTo, 1, 204)
