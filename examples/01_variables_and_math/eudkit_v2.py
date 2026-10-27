"""Example 01: Variables & Arithmetic — Modern eudkit (v2).

Demonstrates:
- Clean Compilation Session (no global state, isolated per-instance)
- Unified variable declaration (automatic storage lowering: 4B memory cell vs 72B VProc)
- Natural Python arithmetic expressions (+, -, *, //, &, |, ^)
- Direct action emission without DoActions boilerplate
- Inspection and deterministic compilation report
"""

from pathlib import Path
import eudkit
from eudkit import Player1, SetDeaths, SetTo

REPO_ROOT = Path(__file__).resolve().parent.parent.parent
BASEMAP_PATH = REPO_ROOT / "tests" / "outputmap" / "basemap" / "basemap_strx.scx"


def main():
    # 1. Initialize isolated Compilation Session
    session = eudkit.Session(BASEMAP_PATH)

    # 2. Declare variables.
    # The Rust compiler analyzes variable usage:
    # Variables that are only read or assigned constants are lowered to 4-byte ByteBuf cells (0 VProcs).
    # Variables requiring dynamic assignment use scratch-reused VProc slots.
    a = session.var(10, name="a")
    b = session.var(20, name="b")
    c = session.var(5, name="c")
    d = session.var(0, name="d")

    # 3. Define game loop routine using standard Python syntax
    @session.fn
    def game_loop():
        # Natural compound expressions are compiled into optimized trigger chains
        d = (a + b) - c + (b & 0xFF)

        # Direct, typed action calls without DoActions boilerplate
        SetDeaths(Player1, SetTo, d, "Terran Marine")

    # 4. Compile map to output
    out_path = Path(__file__).parent / "output_v2.scx"
    report = session.compile(out_path)

    print("=== eudkit (v2) Compilation Report ===")
    print(f"Total Triggers:   {report.trigger_count}")
    print(f"Payload Size:     {report.payload_size} bytes")
    print(f"VProc Count:      {report.vproc_count}")
    print(f"Output SCX Map:   {out_path}")


if __name__ == "__main__":
    main()
