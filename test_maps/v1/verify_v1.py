"""Legacy eudplib (v1) verification map script.

Intent:
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

from eudplib import *

BASEMAP_PATH = REPO_ROOT / "tests" / "outputmap" / "basemap" / "basemap_strx.scx"
OUTPUT_PATH = Path(__file__).resolve().parent / "verify_v1.scx"


def main():
    LoadMap(str(BASEMAP_PATH))
    CompressPayload(True)

    @EUDFunc
    def v1_mainloop():
        counter = EUDVariable(0)

        # Initial startup actions (executes once on frame 0)
        DoActions(
            DisplayText("=== [EUDPLIB V1 TEST MAP] ==="),
            SetResources(P1, Add, 5000, Ore),
            SetResources(P1, Add, 2500, Gas),
        )

        # Main game loop
        if EUDInfLoop()():
            counter += 1

            if EUDIf()(counter >= 240):
                DoActions(
                    counter.SetNumber(0),
                    DisplayText("[v1] Counter reached 240 ticks!"),
                    SetResources(P1, Add, 100, Ore),
                )
            EUDEndIf()

            EUDDoEvents()
        EUDEndInfLoop()

    SaveMap(str(OUTPUT_PATH), v1_mainloop)
    print(f"Successfully compiled v1 map: {OUTPUT_PATH} ({OUTPUT_PATH.stat().st_size:,} bytes)")


if __name__ == "__main__":
    main()
