"""Scenario 17: Unit Snapshot and Restore — Modern eudkit (v2).

Ideal Intent:
Capture and restore a complete 84-dword (336-byte) CUnit structure:
1. Snapshot request (`snapshot_flag == 1`):
   - Copy 84 dwords from CUnit slot 0 (`0x59CCA8`) into a snapshot buffer.
   - Verify unit type integrity via `wread_epd`.
   - Set `snapshot_valid = 1`, increment `snapshots_done`, clear `snapshot_flag`.
2. Restore request (`restore_flag == 1`):
   - If a valid snapshot exists (`snapshot_valid == 1`), copy the 84 dwords back to slot 0.
   - Increment `restores_done`, clear `restore_flag`.
"""

from pathlib import Path
import eudkit
from eudkit import (
    Array,
    EPD,
    dwread_epd,
    dwwrite_epd,
    wread_epd,
)

REPO_ROOT = Path(__file__).resolve().parents[3]
BASEMAP_PATH = REPO_ROOT / "tests" / "outputmap" / "basemap" / "basemap_strx.scx"

CUNIT_TABLE = 0x59CCA8
UNIT_EPD = EPD(CUNIT_TABLE)
SNAPSHOT_DWORDS = 84
SNAPSHOT_BUF_EPD = EPD(0x6FE000)
UNIT_TYPE_OFFSET = 0x64 // 4


def build(session: eudkit.Session):
    snapshot_flag = session.var(0, name="snapshot_flag")
    restore_flag = session.var(0, name="restore_flag")
    snapshot_valid = session.var(0, name="snapshot_valid")
    snapshots_done = session.var(0, name="snapshots_done")
    restores_done = session.var(0, name="restores_done")
    snapshot_type = session.var(0, name="snapshot_type")
    i = session.var(0, name="i")

    snapshot_buf = Array(SNAPSHOT_DWORDS, epd=SNAPSHOT_BUF_EPD, session=session)

    @session.fn
    def game_loop():
        # Direction 1: Snapshot CUnit -> Buffer
        if snapshot_flag == 1:
            i = 0
            while i < SNAPSHOT_DWORDS:
                dwwrite_epd(SNAPSHOT_BUF_EPD + i, dwread_epd(UNIT_EPD + i))
                i += 1

            snapshot_type = wread_epd(SNAPSHOT_BUF_EPD + UNIT_TYPE_OFFSET, 0)
            snapshot_valid = 1
            snapshots_done += 1
            snapshot_flag = 0

        # Direction 2: Restore Buffer -> CUnit
        if restore_flag == 1:
            if snapshot_valid == 1:
                i = 0
                while i < SNAPSHOT_DWORDS:
                    dwwrite_epd(UNIT_EPD + i, dwread_epd(SNAPSHOT_BUF_EPD + i))
                    i += 1
                restores_done += 1
            restore_flag = 0

    return {
        "snapshot_flag": snapshot_flag,
        "restore_flag": restore_flag,
        "snapshot_valid": snapshot_valid,
        "snapshots_done": snapshots_done,
        "restores_done": restores_done,
        "snapshot_type": snapshot_type,
    }


def main():
    session = eudkit.Session(BASEMAP_PATH)
    build(session)
    out_path = Path(__file__).parent / "output_v2.scx"
    report = session.compile(out_path)
    print("Scenario 17 (v2) compiled successfully:")
    print(f"  Triggers: {report.trigger_count}")
    print(f"  Payload:  {report.payload_size} bytes")
    print(f"  VProcs:   {report.vproc_count}")
    return report


if __name__ == "__main__":
    main()
