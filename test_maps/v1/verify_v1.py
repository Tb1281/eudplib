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

import eudplib as ep

BASEMAP_PATH = REPO_ROOT / "tests" / "outputmap" / "basemap" / "basemap_strx.scx"
OUTPUT_PATH = Path(__file__).resolve().parent / "verify_v1.scx"


def main():
    ep.CompressPayload(True)
    ep.LoadMap(str(BASEMAP_PATH))

    @ep.EUDFunc
    def v1_mainloop():
        counter = ep.EUDVariable(0)

        # Initial startup actions
        ep.DisplayText("=== [EUDPLIB V1 TEST MAP] ===")
        ep.SetResources(ep.P1, ep.Add, 5000, ep.Ore)
        ep.SetResources(ep.P1, ep.Add, 2500, ep.Gas)

        # Main game loop
        if ep.EUDWhile()(True):
            counter += 1

            if ep.EUDIf()(counter >= 240):
                counter << 0
                ep.DisplayText("[v1] Counter reached 240 ticks!")
                ep.SetResources(ep.P1, ep.Add, 100, ep.Ore)
            ep.EUDEndIf()

            ep.EUDDoEvents()
        ep.EUDEndWhile()

    ep.SaveMap(str(OUTPUT_PATH), v1_mainloop)
    print(f"Successfully compiled v1 map: {OUTPUT_PATH} ({OUTPUT_PATH.stat().st_size:,} bytes)")


if __name__ == "__main__":
    main()
