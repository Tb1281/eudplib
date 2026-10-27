"""Example 02: Control Flow — Modern eudkit (v2).

Demonstrates:
- Native Python control structures: if, elif, else
- Native Python loops: while, continue, break
- Compile-time loop unrolling: for i in range(...)
- Action coalescing (merging up to 64 consecutive actions into single trigger blocks)
"""

from pathlib import Path
import eudkit
from eudkit import Add, Ore, Player1, SetDeaths, SetResources

REPO_ROOT = Path(__file__).resolve().parent.parent.parent
BASEMAP_PATH = REPO_ROOT / "tests" / "outputmap" / "basemap" / "basemap_strx.scx"


def main():
    session = eudkit.Session(BASEMAP_PATH)
    a = session.var(0, name="a")
    b = session.var(20, name="b")
    c = session.var(0, name="c")

    @session.fn
    def game_loop():
        # 1. Compile-time loop unrolling with action coalescing
        for i in range(10):
            a += 1
            SetDeaths(Player1, Add, 1, "Terran Marine")

        # 2. Idiomatic if / elif / else branches
        # The Rust compiler analyzes branch blocks and inverts conditions in-place
        # to jump directly to destination blocks without extra trampoline triggers.
        if a >= 30:
            c = 100
            SetResources(Player1, Add, 500, Ore)
        elif a == 15:
            c = 50
        else:
            c = 10

        # 3. Standard while loop with continue and break
        while b >= 1:
            b -= 1
            if b == 5:
                a += 10
                continue
            if a >= 200:
                break
            a += 2

    out_path = Path(__file__).parent / "output_v2.scx"
    report = session.compile(out_path)

    print("=== eudkit (v2) Control Flow Compilation Report ===")
    print(f"Total Triggers:   {report.trigger_count}")
    print(f"Payload Size:     {report.payload_size} bytes")
    print(f"Output SCX Map:   {out_path}")


if __name__ == "__main__":
    main()
