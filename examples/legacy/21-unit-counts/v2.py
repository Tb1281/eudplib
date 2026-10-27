"""Scenario 21: Existing vs Completed Unit Counts — Modern eudkit (v2).

Ideal Intent:
Evaluate unit counts for Player 1 across both existing (under construction / morphed)
and completed states:
1. Terran Marines existing >= 1 -> flag 208 (`Command`).
2. Terran Siege Tanks existing >= 3 -> flag 209 (`Command`).
3. Terran Marines completed >= 1 -> flag 210 (direct query on Completed Table at `0x584DE4`).
4. Terran Siege Tanks completed >= 3 -> flag 211.
"""

from pathlib import Path
import eudkit
from eudkit import (
    AtLeast,
    Command,
    Memory,
    Player1,
    SetDeaths,
    SetTo,
)

REPO_ROOT = Path(__file__).resolve().parents[3]
BASEMAP_PATH = REPO_ROOT / "tests" / "outputmap" / "basemap" / "basemap_strx.scx"

COMPLETED_BASE = 0x584DE4


def counts_addr(base: int, player: int, unit: int) -> int:
    return base + 48 * unit + 4 * player


def build(session: eudkit.Session):
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

    return {}


def main():
    session = eudkit.Session(BASEMAP_PATH)
    build(session)
    out_path = Path(__file__).parent / "output_v2.scx"
    report = session.compile(out_path)
    print("Scenario 21 (v2) compiled successfully:")
    print(f"  Triggers: {report.trigger_count}")
    print(f"  Payload:  {report.payload_size} bytes")
    print(f"  VProcs:   {report.vproc_count}")
    return report


if __name__ == "__main__":
    main()
