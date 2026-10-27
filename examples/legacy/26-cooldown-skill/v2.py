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

from pathlib import Path
import eudkit
from eudkit import (
    AtLeast,
    Bring,
    Player1,
    Player8,
    SetDeaths,
    Subtract,
    after,
)

REPO_ROOT = Path(__file__).resolve().parents[3]
BASEMAP_PATH = REPO_ROOT / "tests" / "outputmap" / "basemap" / "basemap_strx.scx"

COOLDOWN_LOOPS = 300
LOC_CAST_ZONE = 3


def build(session: eudkit.Session):
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

    return {
        "ready": ready,
        "casts": casts,
        "wasted": wasted,
        "kills_credited": kills_credited,
    }


def main():
    session = eudkit.Session(BASEMAP_PATH)
    build(session)
    out_path = Path(__file__).parent / "output_v2.scx"
    report = session.compile(out_path)
    print("Scenario 26 (v2) compiled successfully:")
    print(f"  Triggers: {report.trigger_count}")
    print(f"  Payload:  {report.payload_size} bytes")
    print(f"  VProcs:   {report.vproc_count}")
    return report


if __name__ == "__main__":
    main()
