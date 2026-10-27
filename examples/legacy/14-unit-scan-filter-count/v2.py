"""Scenario 14: Unit Table Scan and Filter/Count — Modern eudkit (v2).

Ideal Intent:
Perform a full scan over all 1700 CUnit slots every frame:
1. Identify all active live units (`unitType != 228`).
2. Filter for Player 1 units (`owner == 0`).
3. Count Player 1 Terran Marines (`unitType == 0`).
4. Categorize Marine health:
   - HP >= 256 -> `p1_veterans`
   - HP < 256 -> `p1_wounded`
5. Update scoreboard flags when veteran/wounded milestones are met.
"""

from pathlib import Path
import eudkit
from eudkit import (
    EPD,
    Player1,
    SetDeaths,
    SetTo,
    bread_epd,
    dwread_epd,
    wread_epd,
)

REPO_ROOT = Path(__file__).resolve().parents[3]
BASEMAP_PATH = REPO_ROOT / "tests" / "outputmap" / "basemap" / "basemap_strx.scx"

CUNIT_BASE_EPD = EPD(0x59CCA8)
STRIDE = 84
NONE_TYPE = 228
MARINE = 0


def build(session: eudkit.Session):
    total_live = session.var(0, name="total_live")
    p1_veterans = session.var(0, name="p1_veterans")
    p1_wounded = session.var(0, name="p1_wounded")
    p1_marines = session.var(0, name="p1_marines")
    i = session.var(0, name="i")
    slot_epd = session.var(0, name="slot_epd")
    unit_type = session.var(0, name="unit_type")
    owner = session.var(0, name="owner")
    hp = session.var(0, name="hp")

    @session.fn
    def game_loop():
        total_live = 0
        p1_veterans = 0
        p1_wounded = 0
        p1_marines = 0

        # Structured single-pass scan across all 1700 table slots
        i = 0
        while i < 1700:
            slot_epd = CUNIT_BASE_EPD + i * STRIDE
            unit_type = wread_epd(slot_epd + 25, 0)
            if unit_type != NONE_TYPE:
                total_live += 1
                owner = bread_epd(slot_epd + 19, 0)
                if owner == 0:
                    if unit_type == MARINE:
                        p1_marines += 1
                        hp = dwread_epd(slot_epd + 2)
                        if hp >= 256:
                            p1_veterans += 1
                        else:
                            p1_wounded += 1
            i += 1

        # Scoreboard milestone indicators
        if p1_veterans >= 10:
            SetDeaths(Player1, SetTo, 1, 200)
        if p1_wounded >= 1:
            SetDeaths(Player1, SetTo, 1, 201)

    return {
        "total_live": total_live,
        "p1_veterans": p1_veterans,
        "p1_wounded": p1_wounded,
        "p1_marines": p1_marines,
    }


def main():
    session = eudkit.Session(BASEMAP_PATH)
    build(session)
    out_path = Path(__file__).parent / "output_v2.scx"
    report = session.compile(out_path)
    print("Scenario 14 (v2) compiled successfully:")
    print(f"  Triggers: {report.trigger_count}")
    print(f"  Payload:  {report.payload_size} bytes")
    print(f"  VProcs:   {report.vproc_count}")
    return report


if __name__ == "__main__":
    main()
