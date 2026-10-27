"""Scenario 20: Bring Condition Area Count — Modern eudkit (v2).

Ideal Intent:
Evaluate player unit presence inside a target location using native engine queries:
1. Test if Player 1 has at least 1 Terran Marine in location 1 -> flag 205.
2. Test if Player 1 has at least 3 Terran Marines in location 1 -> flag 206.
3. Test if Player 1 has at least 5 Terran Marines in location 1 -> flag 207.
"""

from pathlib import Path
import eudkit
from eudkit import (
    AtLeast,
    Bring,
    Player1,
    SetDeaths,
    SetTo,
)

REPO_ROOT = Path(__file__).resolve().parents[3]
BASEMAP_PATH = REPO_ROOT / "tests" / "outputmap" / "basemap" / "basemap_strx.scx"


def build(session: eudkit.Session):
    @session.fn
    def game_loop():
        # High-level typed Bring conditions evaluated natively by StarCraft's trigger engine
        if Bring(Player1, AtLeast, 1, "Terran Marine", 1):
            SetDeaths(Player1, SetTo, 1, 205)

        if Bring(Player1, AtLeast, 3, "Terran Marine", 1):
            SetDeaths(Player1, SetTo, 1, 206)

        if Bring(Player1, AtLeast, 5, "Terran Marine", 1):
            SetDeaths(Player1, SetTo, 1, 207)

    return {}


def main():
    session = eudkit.Session(BASEMAP_PATH)
    build(session)
    out_path = Path(__file__).parent / "output_v2.scx"
    report = session.compile(out_path)
    print("Scenario 20 (v2) compiled successfully:")
    print(f"  Triggers: {report.trigger_count}")
    print(f"  Payload:  {report.payload_size} bytes")
    print(f"  VProcs:   {report.vproc_count}")
    return report


if __name__ == "__main__":
    main()
