"""Scenario 17 (legacy) — snapshot / restore one unit's CUnit record.

Intent: guard the unit in CUnit slot 0. When a snapshot is requested,
copy its whole 336-byte record (84 dwords) into a private buffer;
when a restore is requested, copy the record back (reviving the
unit's exact state). Count both operations; read the unit type back
from the snapshot as a sanity check.
Legacy idiom: f_repmovsd_epd — the CP-cursor block mover riding the
import-time readtable/CP engines.
"""

from eudkit import *  # noqa: F403

CUNIT_TABLE = 0x59CCA8
UNIT_EPD = EPD(CUNIT_TABLE)  # slot 0's record, as a Deaths EPD
SNAPSHOT_DWORDS = 84  # one CUnit record: 336 bytes / 4
UNIT_TYPE_EPD_OFFSET = 0x64 // 4  # unitType word at +0x64

snapshot_buf = Db(336)
snapshot_epd = EPD(snapshot_buf)

snapshot_flag = EUDVariable()  # 1 = take a snapshot now
restore_flag = EUDVariable()  # 1 = restore from the snapshot now
snapshot_valid = EUDVariable()  # 1 once a snapshot exists
snapshots_done = EUDVariable()
restores_done = EUDVariable()
snapshot_type = EUDVariable()  # unitType read back from the copy


def run():
    global snapshot_flag, restore_flag, snapshot_valid
    global snapshots_done, restores_done, snapshot_type
    if EUDIf()(snapshot_flag == 1):
        # Copy direction 1: unit record -> private buffer.
        f_repmovsd_epd(snapshot_epd, UNIT_EPD, SNAPSHOT_DWORDS)
        # Sanity check: the copied unitType word (a real read).
        snapshot_type << f_wread_epd(snapshot_epd + UNIT_TYPE_EPD_OFFSET, 0)
        snapshot_valid << 1
        snapshots_done += 1
        snapshot_flag << 0
    EUDEndIf()

    if EUDIf()(restore_flag == 1):
        if EUDIf()(snapshot_valid == 1):
            # Copy direction 2: private buffer -> unit record.
            f_repmovsd_epd(UNIT_EPD, snapshot_epd, SNAPSHOT_DWORDS)
            restores_done += 1
        EUDEndIf()
        restore_flag << 0
    EUDEndIf()
