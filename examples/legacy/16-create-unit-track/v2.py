"""Scenario 16: Unit Creation and Handle Tracking — Modern eudkit (v2).

Ideal Intent:
Every 24 trigger cycles (1 second at Fastest speed):
1. Capture the engine's "next empty unit" pointer from `0x628438` before creation.
2. Spawn 1 Terran Marine for Player 1.
3. Immediately buff the spawned unit's HP to 512.
4. Record its EPD handle and track spawn counts.
5. Set milestone death flag when `spawn_count >= 10`.
"""

from pathlib import Path
import eudkit
from eudkit import (
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

REPO_ROOT = Path(__file__).resolve().parents[3]
BASEMAP_PATH = REPO_ROOT / "tests" / "outputmap" / "basemap" / "basemap_strx.scx"

NEXT_UNIT_EPD = EPD(0x628438)


def build(session: eudkit.Session):
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

    return {
        "spawn_count": spawn_count,
        "last_spawn_epd": last_spawn_epd,
        "last_spawn_hp": last_spawn_hp,
    }


def main():
    session = eudkit.Session(BASEMAP_PATH)
    build(session)
    out_path = Path(__file__).parent / "output_v2.scx"
    report = session.compile(out_path)
    print("Scenario 16 (v2) compiled successfully:")
    print(f"  Triggers: {report.trigger_count}")
    print(f"  Payload:  {report.payload_size} bytes")
    print(f"  VProcs:   {report.vproc_count}")
    return report


if __name__ == "__main__":
    main()
