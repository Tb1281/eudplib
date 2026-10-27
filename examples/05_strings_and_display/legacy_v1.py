"""Example 05: Strings & Display — Legacy eudplib (v1).

Demonstrates:
- f_eprintln and DBString mechanisms
- Runtime XOR decryption loops emitted for all string payloads
- Heavy trigger apparatus (>500 triggers and 170KB+ STRx bloat)
- Non-deterministic string table offsets
"""

from pathlib import Path
import eudplib as ep

REPO_ROOT = Path(__file__).resolve().parent.parent.parent
BASEMAP_PATH = REPO_ROOT / "tests" / "outputmap" / "basemap" / "basemap_strx.scx"


def main():
    ep.LoadMap(str(BASEMAP_PATH))
    ep.CompressPayload(True)

    @ep.EUDFunc
    def v1_display_workload():
        score = ep.EUDVariable(100)
        minerals = ep.EUDVariable(500)
        gas = ep.EUDVariable(250)

        # In legacy v1, f_eprintln builds runtime decryption tables and character copy loops:
        ep.f_eprintln("Status Report: Score = ", score)
        ep.f_eprintln("Resources: Min = ", minerals, " Gas = ", gas)

        # Checking CurrentPlayer requires explicit helper macros:
        if ep.EUDIf()(ep.IsUserCP()):
            ep.f_eprintln("Private Dispatch: Active Player Score = ", score)
        ep.EUDEndIf()

    out_path = Path(__file__).parent / "output_v1.scx"
    ep.SaveMap(str(out_path), v1_display_workload)
    print(f"Legacy v1 strings workload compiled to: {out_path}")


if __name__ == "__main__":
    main()
