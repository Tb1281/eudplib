"""Scenario 19: Spatial Unit Finder (2D Range Query) — Modern eudkit (v2).

Ideal Intent:
Find all units within a bounding box `[target - radius, target + radius]` in 2D space:
1. Walk StarCraft's sorted Unit Finder X array (`0x66FF78`: 3400 entries of `{coord_x, unit_ptr}`).
2. Early exit (`break`) as soon as `coord_x > hi_x` (exploiting sorted order).
3. For entries within `[lo_x, hi_x]`:
   - Resolve unit EPD via `epdread_epd`.
   - Inspect `posY` from CUnit offset `0x2A` (`wread_epd(unit_epd + 10, 1)`).
   - Test if `lo_y <= pos_y <= hi_y`.
   - Record first match and increment `found_count`.
"""

from pathlib import Path
import eudkit
from eudkit import (
    EPD,
    dwread_epd,
    epdread_epd,
    wread_epd,
)

REPO_ROOT = Path(__file__).resolve().parents[3]
BASEMAP_PATH = REPO_ROOT / "tests" / "outputmap" / "basemap" / "basemap_strx.scx"

UNIT_FINDER_X = 0x66FF78
FINDER_BASE_EPD = EPD(UNIT_FINDER_X)
FINDER_ENTRIES = 3400


def build(session: eudkit.Session):
    target_x = session.var(1024, name="target_x")
    target_y = session.var(1024, name="target_y")
    radius = session.var(128, name="radius")
    found_count = session.var(0, name="found_count")
    first_found_epd = session.var(0, name="first_found_epd")

    lo_x = session.var(0, name="lo_x")
    hi_x = session.var(0, name="hi_x")
    lo_y = session.var(0, name="lo_y")
    hi_y = session.var(0, name="hi_y")
    idx = session.var(0, name="idx")
    coord = session.var(0, name="coord")
    unit_epd = session.var(0, name="unit_epd")
    pos_y = session.var(0, name="pos_y")

    @session.fn
    def game_loop():
        found_count = 0
        first_found_epd = 0
        lo_x = target_x - radius
        hi_x = target_x + radius
        lo_y = target_y - radius
        hi_y = target_y + radius

        idx = 0
        while idx < FINDER_ENTRIES:
            coord = dwread_epd(FINDER_BASE_EPD + idx * 2)
            # Binary sweep pruning: sorted array guarantees no further matches
            if coord > hi_x:
                break
            if coord >= lo_x:
                unit_epd = epdread_epd(FINDER_BASE_EPD + idx * 2 + 1)
                pos_y = wread_epd(unit_epd + 10, 1)  # offset 0x2A = dword 10, subp 1
                if pos_y >= lo_y:
                    if pos_y <= hi_y:
                        if found_count == 0:
                            first_found_epd = unit_epd
                        found_count += 1
            idx += 1

    return {
        "target_x": target_x,
        "target_y": target_y,
        "radius": radius,
        "found_count": found_count,
        "first_found_epd": first_found_epd,
    }


def main():
    session = eudkit.Session(BASEMAP_PATH)
    build(session)
    out_path = Path(__file__).parent / "output_v2.scx"
    report = session.compile(out_path)
    print("Scenario 19 (v2) compiled successfully:")
    print(f"  Triggers: {report.trigger_count}")
    print(f"  Payload:  {report.payload_size} bytes")
    print(f"  VProcs:   {report.vproc_count}")
    return report


if __name__ == "__main__":
    main()
