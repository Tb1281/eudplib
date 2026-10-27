"""Example 01: Variables & Arithmetic — Legacy eudplib (v1).

Demonstrates:
- Manual variable declaration (EUDVariable creates a 72-byte VProc trigger slot)
- Bitshift/assignment syntax (<< and += operator overloading)
- Manual action emission (ep.DoActions)
- Global map lifecycle (LoadMap, SaveMap)
"""

from pathlib import Path
import eudplib as ep

REPO_ROOT = Path(__file__).resolve().parent.parent.parent
BASEMAP_PATH = REPO_ROOT / "tests" / "outputmap" / "basemap" / "basemap_strx.scx"


def main():
    # 1. Initialize global map context
    ep.LoadMap(str(BASEMAP_PATH))
    ep.CompressPayload(True)

    # 2. Define main trigger routine
    def v1_workload():
        # EUDVariable allocates 72-byte VProc trigger even for constants
        a = ep.EUDVariable(10)
        b = ep.EUDVariable(20)
        c = ep.EUDVariable(5)
        d = ep.EUDVariable(0)

        # Compound expression requires temporary variable ladders
        # a + b allocates temporary trigger, then - c allocates another
        d << (a + b) - c + (b & 0xFF)

        # Action emission requires DoActions wrapper
        ep.DoActions(ep.SetDeaths(ep.Player1, ep.SetTo, d, "Terran Marine"))

    # 3. Save map with root function (modifies global phase manager)
    out_path = Path(__file__).parent / "output_v1.scx"
    ep.SaveMap(str(out_path), v1_workload)
    print(f"Legacy v1 compiled to: {out_path}")


if __name__ == "__main__":
    main()
