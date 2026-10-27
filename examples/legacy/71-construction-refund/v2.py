# 71 - construction-cancel refund (modern eudkit)
#
# Expresses ideal intent:
# 1. Direct unit-count table reads (0x582324 / 0x584DE4) with EPD sub-dword indexing.
# 2. Construction cancellation delta detection and instant refund writeback.
# 3. Clean session state with zero global singletons.
from pathlib import Path
import eudkit

BASEMAP_PATH = Path(__file__).resolve().parents[3] / "tests" / "outputmap" / "basemap" / "basemap_strx.scx"

P1 = 0
BARRACKS = 111
BARRACKS_COST = 150
ALL_COUNTS = 0x582324
DONE_COUNTS = 0x584DE4
ORE_ADDR = 0x57F0F0


def build(session: eudkit.Session):
    existing = session.var(0)
    completed = session.var(0)
    prev_building = session.var(0)
    prev_completed = session.var(0)
    started_total = session.var(0)
    finished_total = session.var(0)
    cancelled_total = session.var(0)
    refunded_total = session.var(0)

    @session.fn(main_loop=True)
    def game_loop() -> None:
        nonlocal existing, completed, prev_building, prev_completed
        nonlocal started_total, finished_total, cancelled_total, refunded_total

        existing = eudkit.dwread(ALL_COUNTS + 48 * BARRACKS + 4 * P1)
        completed = eudkit.dwread(DONE_COUNTS + 48 * BARRACKS + 4 * P1)
        building = existing - completed

        with session.once():
            prev_building = building
            prev_completed = completed

        d_completed = 0
        if completed > prev_completed:
            d_completed = completed - prev_completed
            finished_total += d_completed

        if building < prev_building:
            d_building = prev_building - building
            if d_building > d_completed:
                cancelled_now = d_building - d_completed
                cancelled_total += cancelled_now
                refund_amount = cancelled_now * BARRACKS_COST
                ore_now = eudkit.dwread(ORE_ADDR + 4 * P1)
                eudkit.dwwrite(ORE_ADDR + 4 * P1, ore_now + refund_amount)
                refunded_total += refund_amount

        if building > prev_building:
            started_total += building - prev_building

        prev_building = building
        prev_completed = completed

    return {
        "existing": existing,
        "completed": completed,
        "started_total": started_total,
        "finished_total": finished_total,
        "cancelled_total": cancelled_total,
        "refunded_total": refunded_total,
    }


def main():
    session = eudkit.Session(BASEMAP_PATH)
    build(session)
    out_path = Path(__file__).parent / "output_v2.scx"
    report = session.compile(out_path)
    if out_path.exists():
        out_path.unlink()
    print("Scenario 71 (v2) compiled successfully:")
    print(f"  Triggers: {report.trigger_count}")
    print(f"  Payload:  {report.payload_size} bytes")
    print(f"  VProcs:   {report.vproc_count}")
    return report


if __name__ == "__main__":
    main()
