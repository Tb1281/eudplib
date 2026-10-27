"""eudkit (v2) verification map script.

Intent (Identical to v1):
1. Frame 0 (First tick):
   - Display banner text
   - Grant 5000 Ore (Minerals) and 2500 Gas to Player 1
2. Every tick:
   - Increment a frame counter
   - Every 240 ticks:
     - Reset counter to 0
     - Display tick milestone notification
     - Grant 100 Ore to Player 1
"""

import sys
from pathlib import Path

# Add repo src to path
REPO_ROOT = Path(__file__).resolve().parent.parent.parent
sys.path.insert(0, str(REPO_ROOT / "src"))

import eudkit
from eudkit import (
    Add,
    DisplayText,
    Gas,
    Ore,
    Player1,
    SetResources,
    once,
    when,
)

BASEMAP_PATH = REPO_ROOT / "tests" / "outputmap" / "basemap" / "basemap_strx.scx"
OUTPUT_PATH = Path(__file__).resolve().parent / "verify_v2.scx"


def main():
    session = eudkit.Session(BASEMAP_PATH)
    counter = session.var(0, name="counter")

    @session.fn
    def v2_mainloop():
        # Initial startup actions (executes once on frame 0)
        with once:
            DisplayText("=== [EUDKIT V2 TEST MAP] ===")
            SetResources(Player1, Add, 5000, Ore)
            SetResources(Player1, Add, 2500, Gas)

        # Main game loop: increment counter every trigger tick
        counter += 1

        # Every 240 ticks: reset and award resources
        with when(counter >= 240):
            counter = 0
            DisplayText("[v2] Counter reached 240 ticks!")
            SetResources(Player1, Add, 100, Ore)

    report = session.compile(OUTPUT_PATH)
    print(
        f"Successfully compiled v2 map: {OUTPUT_PATH} "
        f"({OUTPUT_PATH.stat().st_size:,} bytes, "
        f"triggers: {report.trigger_count}, payload: {report.payload_size:,} bytes)"
    )


if __name__ == "__main__":
    main()
