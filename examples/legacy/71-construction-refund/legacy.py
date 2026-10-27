# 71 - construction-cancel refund (legacy eudplib)
#
# Intent: watch Player 1's Barracks construction through the
# engine's own unit-count tables and refund cancellations:
#   existing  = all Barracks (completed + under construction),
#               from the "all units" count table at 0x582324
#   completed = finished Barracks, from the "completed units"
#               count table at 0x584DE4
# Both tables are indexed [unit][player]: addr = base + 48*unit
# + 4*player -- the same tables a Deaths(player - 8208 / - 5472)
# condition reads, verified against the EUD Book.
#   building  = existing - completed
# Each loop the deltas are compared against last loop's values:
# a building that disappears WITHOUT the completed count rising
# by the same amount was cancelled, and its 150-ore cost goes
# back to the player's mineral dword (0x57F0F0). A building that
# finishes moves building -> completed with no refund. New starts
# simply raise the tracked counts.
#
# Counting form (pilot convention): run() builds the scenario's
# triggers once; benchmarks/pilot/count_legacy.py calls it inside
# a fresh PushTriggerScope and reports the GetTriggerCounter()
# delta. The f_dwread / f_dwwrite call sites ride the import-time
# read/write engines (session-amortized, outside the delta) --
# the pilot RESULTS asymmetry, stated in note.md.
from eudkit import *

P1 = 0
BARRACKS = 111
BARRACKS_COST = 150
ALL_COUNTS = 0x582324      # existing (all) unit counts table
DONE_COUNTS = 0x584DE4     # completed unit counts table
ORE_ADDR = 0x57F0F0        # minerals array, + 4 * player

existing = EUDVariable()
completed = EUDVariable()
prev_building = EUDVariable()
prev_completed = EUDVariable()
building = EUDVariable()        # scratch: existing - completed
d_building = EUDVariable()      # scratch: prev_building - building
d_completed = EUDVariable()     # scratch: completed - prev_completed
cancelled_now = EUDVariable()   # scratch: refunds due this loop
ore_now = EUDVariable()         # scratch: current mineral value
started_total = EUDVariable()
finished_total = EUDVariable()
cancelled_total = EUDVariable()
refunded_total = EUDVariable()
first_loop = EUDVariable()


def run():
    global existing, completed, building, prev_building
    global prev_completed, d_building, d_completed, cancelled_now
    global ore_now, started_total, finished_total, cancelled_total
    global refunded_total, first_loop

    existing << f_dwread(ALL_COUNTS + 48 * BARRACKS + 4 * P1)
    completed << f_dwread(DONE_COUNTS + 48 * BARRACKS + 4 * P1)
    building << existing - completed

    # First loop only seeds the trackers (no history to diff).
    if EUDIf()(first_loop == 0):
        first_loop << 1
        prev_building << building
        prev_completed << completed
    EUDEndIf()

    if EUDIf()(first_loop == 1):
        # Completions register first: they explain shrinkages.
        if EUDIf()(completed > prev_completed):
            d_completed << completed - prev_completed
            finished_total += d_completed
        if EUDElse()():
            d_completed << 0
        EUDEndIf()

        if EUDIf()(building < prev_building):
            d_building << prev_building - building
            # The unexplained shrinkage is cancellations.
            if EUDIf()(d_building > d_completed):
                cancelled_now << d_building - d_completed
                cancelled_total += cancelled_now
                ore_now << f_dwread(ORE_ADDR + 4 * P1)
                f_dwwrite(ORE_ADDR + 4 * P1,
                          ore_now + cancelled_now * BARRACKS_COST)
                refunded_total += cancelled_now * BARRACKS_COST
            EUDEndIf()
        EUDEndIf()

        if EUDIf()(building > prev_building):
            started_total += building - prev_building
        EUDEndIf()

        prev_building << building
        prev_completed << completed
    EUDEndIf()
