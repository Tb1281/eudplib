"""Example 04: Memory & Game Structs — Modern eudkit (v2).

Demonstrates:
- Typed CUnit wrapper with named engine fields (hp, pos, posX, posY, order)
- Struct helpers (unit.set_invincible(), unit.orderTargetPos)
- Static and dynamic Arrays (Array, VArray) with indexing syntax
- Deduplicated ReadTable: all memory reads share a single unified bit-ladder trigger pool
"""

from pathlib import Path
import eudkit
from eudkit import Array, CUnit, VArray, dwread_epd, epdread_epd

REPO_ROOT = Path(__file__).resolve().parent.parent.parent
BASEMAP_PATH = REPO_ROOT / "tests" / "outputmap" / "basemap" / "basemap_strx.scx"


def main():
    session = eudkit.Session(BASEMAP_PATH)

    # 1. Typed Game Struct wrapper for unit index 0
    unit = CUnit(0, session=session)

    # 2. Typed Arrays mapped to memory
    arr = Array(10, epd=0x58A364 // 4, session=session)
    varr = VArray(8, epd=0x58A364 // 4, session=session)

    idx = session.var(2, name="idx")
    res = session.var(0, name="res")

    @session.fn
    def game_loop():
        # High-level struct property reads
        current_hp = unit.hp
        current_pos = unit.pos
        order = unit.order

        # Modifying struct fields lowers to single SetDeaths / SetMemory actions
        unit.hp = 500
        unit.pos = 0x02000200
        unit.set_invincible()

        # Dynamic array access
        arr[1] = 500
        varr[0] = 777
        arr[idx] = 1234
        varr[idx] = 5678
        res = arr[idx]

    out_path = Path(__file__).parent / "output_v2.scx"
    report = session.compile(out_path)

    print("=== eudkit (v2) Memory & Entities Compilation Report ===")
    print(f"Total Triggers:   {report.trigger_count}")
    print(f"Payload Size:     {report.payload_size} bytes")
    print(f"Output SCX Map:   {out_path}")


if __name__ == "__main__":
    main()
