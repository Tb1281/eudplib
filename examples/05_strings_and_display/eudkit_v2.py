"""Example 05: Strings & Display — Modern eudkit (v2).

Demonstrates:
- Deduplicated MapString resolution at compile time
- Residue-stride CPString streaming actions (mod 12 unit-stride grouping)
- Static STRx relocation (0x191943C8)
- f_eprintln template formatting (%d, %s) with 0 runtime decryption overhead
- IsUserCP condition checking
"""

from pathlib import Path
import eudkit
from eudkit import IsUserCP, f_eprintln

REPO_ROOT = Path(__file__).resolve().parent.parent.parent
BASEMAP_PATH = REPO_ROOT / "tests" / "outputmap" / "basemap" / "basemap_strx.scx"


def main():
    session = eudkit.Session(BASEMAP_PATH)

    # 1. Deduplicated MapString:
    # Identical strings registered in session scope share the exact same STRx table offset
    ms1 = session.map_string("Mission Objective: Destroy Enemy Nexus")
    ms2 = session.map_string("Mission Objective: Destroy Enemy Nexus")
    assert ms1.id == ms2.id

    score = session.var(100, name="score")
    minerals = session.var(500, name="minerals")
    gas = session.var(250, name="gas")

    @session.fn
    def game_loop():
        # 2. Template string formatting with residue-stride CPString writes:
        # Instead of 150KB runtime loops, eudkit streams characters directly into the
        # StarCraft text buffer using grouped unit-stride CurrentPlayer writes.
        f_eprintln("Status Report: Score = %d", score)
        f_eprintln("Resources: Min = %d, Gas = %d", minerals, gas)

        # 3. Local player filtering
        if IsUserCP():
            f_eprintln("Private Dispatch: Active Player Score = %d", score)

    out_path = Path(__file__).parent / "output_v2.scx"
    report = session.compile(out_path)

    print("=== eudkit (v2) Strings Compilation Report ===")
    print(f"Total Triggers:   {report.trigger_count}")
    print(f"Payload Size:     {report.payload_size} bytes")
    print(f"Output SCX Map:   {out_path}")


if __name__ == "__main__":
    main()
