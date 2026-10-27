"""Scenario 03: Block Copy (CUnit Snapshot) — Modern eudkit (v2).

Ideal Intent:
Whenever a snapshot request is triggered (`request_flag == 1`), copy one unit's
complete CUnit record (336 bytes = 84 dwords) from dynamic `source_epd` into a
designated snapshot buffer.
Bookkeeping:
- Set reentrancy guard (`copy_busy`)
- Copy all 84 dwords
- Read back and verify the unitType field (offset +0x64 = +25 dwords)
- Record last source EPD, increment snapshot completion counter, and release flags.
"""

from pathlib import Path
import eudkit
from eudkit import (
    Array,
    EPD,
    dwread_epd,
    dwwrite_epd,
)

REPO_ROOT = Path(__file__).resolve().parents[3]
BASEMAP_PATH = REPO_ROOT / "tests" / "outputmap" / "basemap" / "basemap_strx.scx"

SNAPSHOT_DWORDS = 84
SNAPSHOT_BUF_EPD = EPD(0x6FE000)
UNIT_TYPE_OFFSET = 0x64 // 4


def build(session: eudkit.Session):
    request_flag = session.var(0, name="request_flag")
    source_epd = session.var(EPD(0x59CCA8), name="source_epd")
    snapshots_done = session.var(0, name="snapshots_done")
    last_source_epd = session.var(0, name="last_source_epd")
    copy_busy = session.var(0, name="copy_busy")
    snapshot_type = session.var(0, name="snapshot_type")
    i = session.var(0, name="i")

    snapshot_buf = Array(SNAPSHOT_DWORDS, epd=SNAPSHOT_BUF_EPD, session=session)

    @session.fn
    def game_loop():
        if request_flag == 1:
            if copy_busy == 0:
                copy_busy = 1

                # Clean structured block copy loop
                i = 0
                while i < SNAPSHOT_DWORDS:
                    dwwrite_epd(SNAPSHOT_BUF_EPD + i, dwread_epd(source_epd + i))
                    i += 1

                # Read back unitType for verification
                snapshot_type = dwread_epd(SNAPSHOT_BUF_EPD + UNIT_TYPE_OFFSET)

                # Finalize bookkeeping
                last_source_epd = source_epd
                snapshots_done += 1
                request_flag = 0
                copy_busy = 0

    return {
        "request_flag": request_flag,
        "source_epd": source_epd,
        "snapshots_done": snapshots_done,
        "last_source_epd": last_source_epd,
        "copy_busy": copy_busy,
        "snapshot_type": snapshot_type,
        "snapshot_buf": snapshot_buf,
    }


def main():
    session = eudkit.Session(BASEMAP_PATH)
    build(session)
    out_path = Path(__file__).parent / "output_v2.scx"
    report = session.compile(out_path)
    print("Scenario 03 (v2) compiled successfully:")
    print(f"  Triggers: {report.trigger_count}")
    print(f"  Payload:  {report.payload_size} bytes")
    print(f"  VProcs:   {report.vproc_count}")
    return report


if __name__ == "__main__":
    main()
