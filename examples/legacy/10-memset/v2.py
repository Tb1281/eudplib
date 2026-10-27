"""Scenario 10: Memset (Buffer Clear) — Modern eudkit (v2).

Ideal Intent:
Clear a 64-dword scratch region on demand:
1. Detect request flag (`clear_flag != 0`).
2. Iterate through 64 dwords, setting each element to 0 via direct memory write.
3. Reset request flag to 0 and increment completion counter (`clears_done += 1`).
"""

from pathlib import Path
import eudkit
from eudkit import (
    EPD,
    dwwrite_epd,
)

REPO_ROOT = Path(__file__).resolve().parents[3]
BASEMAP_PATH = REPO_ROOT / "tests" / "outputmap" / "basemap" / "basemap_strx.scx"

SCRATCH_EPD = EPD(0x6FE000)


def build(session: eudkit.Session):
    clear_flag = session.var(0, name="clear_flag")
    idx = session.var(0, name="idx")
    clears_done = session.var(0, name="clears_done")

    @session.fn
    def game_loop():
        if clear_flag != 0:
            idx = 0
            while idx < 64:
                dwwrite_epd(SCRATCH_EPD + idx, 0)
                idx += 1

            clear_flag = 0
            clears_done += 1

    return {
        "clear_flag": clear_flag,
        "idx": idx,
        "clears_done": clears_done,
    }


def main():
    session = eudkit.Session(BASEMAP_PATH)
    build(session)
    out_path = Path(__file__).parent / "output_v2.scx"
    report = session.compile(out_path)
    print("Scenario 10 (v2) compiled successfully:")
    print(f"  Triggers: {report.trigger_count}")
    print(f"  Payload:  {report.payload_size} bytes")
    print(f"  VProcs:   {report.vproc_count}")
    return report


if __name__ == "__main__":
    main()
