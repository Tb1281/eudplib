"""Scenario 11: Memory Compare (Buffer Equality) — Modern eudkit (v2).

Ideal Intent:
Compare two 4-dword (16-byte) configuration buffers (primary and shadow) for equality:
1. Check first half (dwords 0 and 1) and second half (dwords 2 and 3).
2. Raise `resync_flag = 1` if ANY dword differs between buffers.
3. Identify which half diverged for diagnostics (`diff_half = 1` for first half, `2` for second half, `0` if equal).
"""

from pathlib import Path
import eudkit
from eudkit import (
    EPD,
    dwread_epd,
)

REPO_ROOT = Path(__file__).resolve().parents[3]
BASEMAP_PATH = REPO_ROOT / "tests" / "outputmap" / "basemap" / "basemap_strx.scx"

BUF_A_EPD = EPD(0x58A364 + 0x800)
BUF_B_EPD = EPD(0x58A364 + 0x840)


def build(session: eudkit.Session):
    resync_flag = session.var(0, name="resync_flag")
    diff_half = session.var(0, name="diff_half")
    diff_first = session.var(0, name="diff_first")
    diff_second = session.var(0, name="diff_second")

    @session.fn
    def game_loop():
        diff_first = 0
        diff_second = 0

        # First half compare (dwords 0 and 1)
        if dwread_epd(BUF_A_EPD + 0) != dwread_epd(BUF_B_EPD + 0):
            diff_first = 1
        if dwread_epd(BUF_A_EPD + 1) != dwread_epd(BUF_B_EPD + 1):
            diff_first = 1

        # Second half compare (dwords 2 and 3)
        if dwread_epd(BUF_A_EPD + 2) != dwread_epd(BUF_B_EPD + 2):
            diff_second = 1
        if dwread_epd(BUF_A_EPD + 3) != dwread_epd(BUF_B_EPD + 3):
            diff_second = 1

        # Overall equality verdict
        if diff_first != 0 or diff_second != 0:
            resync_flag = 1
        else:
            resync_flag = 0

        # Divergence localization
        if diff_first != 0:
            diff_half = 1
        elif diff_second != 0:
            diff_half = 2
        else:
            diff_half = 0

    return {
        "resync_flag": resync_flag,
        "diff_half": diff_half,
        "diff_first": diff_first,
        "diff_second": diff_second,
    }


def main():
    session = eudkit.Session(BASEMAP_PATH)
    build(session)
    out_path = Path(__file__).parent / "output_v2.scx"
    report = session.compile(out_path)
    print("Scenario 11 (v2) compiled successfully:")
    print(f"  Triggers: {report.trigger_count}")
    print(f"  Payload:  {report.payload_size} bytes")
    print(f"  VProcs:   {report.vproc_count}")
    return report


if __name__ == "__main__":
    main()
