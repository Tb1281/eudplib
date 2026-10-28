"""LEGACY block copy (pilot 3).

Same intent as ours_3_block_copy.js: snapshot one unit's CUnit
record (336 bytes = 84 dwords) into a private buffer whenever a
"snapshot requested" flag is raised, using f_repmovsd_epd (the
CP-cursor block mover). Bookkeeping: count completed snapshots,
latch the source EPD used, and clear the request flag. The source
EPD is dynamic (whichever unit was selected), so the copy call is
parameterized at game time.
"""

from eudkit import *

SNAPSHOT_DWORDS = 84  # one CUnit record: 336 bytes / 4

# Snapshot destination: a private 84-dword buffer.
snapshot_buf = Db(336)
snapshot_epd = EPD(snapshot_buf)

# CUnit field offset in EPD (dword) units, for the post-copy check.
UNIT_TYPE_EPD_OFFSET = 0x64 // 4  # unitType word at +0x64

# Request/state variables (written by other code in a real map).
request_flag = EUDVariable()     # 1 = snapshot requested
source_epd = EUDVariable()       # EPD of the unit to snapshot
snapshots_done = EUDVariable()   # completed snapshots
last_source_epd = EUDVariable()  # source used by the last snapshot
copy_busy = EUDVariable()        # reentrancy guard
snapshot_type = EUDVariable()    # unitType read back from the copy


def run():
    global snapshots_done, last_source_epd, copy_busy, request_flag
    global snapshot_type
    # Only work when a snapshot was requested and none is running.
    if EUDIf()(request_flag == 1):
        if EUDIf()(copy_busy == 0):
            copy_busy << 1
            # The block copy itself: dst EPD, src EPD, dword count.
            # CP is the source cursor; the read destination is the
            # buffer cursor; both advance inside the mover.
            f_repmovsd_epd(snapshot_epd, source_epd, SNAPSHOT_DWORDS)
            # Sanity check: read the copied unitType back out of the
            # snapshot (the VALUE is needed here, so a real read).
            snapshot_type << f_dwread_epd(snapshot_epd + UNIT_TYPE_EPD_OFFSET)
            # Bookkeeping after the copy completes.
            last_source_epd << source_epd
            snapshots_done += 1
            request_flag << 0
            copy_busy << 0
        EUDEndIf()
    EUDEndIf()
