# 69 - leaderboard: sort player scores, show top 3 (modern eudkit)
#
# Expresses ideal intent:
# 1. Consolidated `StringBuffer` API for single dynamic string architecture (ADR 0004).
# 2. Declarative refresh interval via `with session.every(240):`.
# 3. Competition ranking with pairwise score comparison and formatted display.
from pathlib import Path
import eudkit

BASEMAP_PATH = Path(__file__).resolve().parents[3] / "tests" / "outputmap" / "basemap" / "basemap_strx.scx"

CUSTOM_SCORE_ADDR = 0x5822F4
REFRESH_PERIOD = 240
PLAYERS = 8


def build(session: eudkit.Session):
    line_bufs = [session.string_buffer(48) for _ in range(3)]

    scores = [session.var(0) for _ in range(PLAYERS)]
    ranks = [session.var(0) for _ in range(PLAYERS)]
    refreshes = session.var(0)
    leader = session.var(0)
    shown = [session.var(0) for _ in range(3)]

    @session.fn(main_loop=True)
    def game_loop() -> None:
        nonlocal leader, refreshes

        with session.every(REFRESH_PERIOD):
            for p in range(PLAYERS):
                scores[p] = eudkit.dwread(CUSTOM_SCORE_ADDR + 4 * p)

            for p in range(PLAYERS):
                ranks[p] = 1
                for q in range(PLAYERS):
                    if q != p:
                        if scores[q] > scores[p]:
                            ranks[p] += 1

            leader = 0
            for p in range(PLAYERS):
                if ranks[p] == 1 and leader == 0:
                    leader = p + 1

            for r in range(3):
                shown[r] = 0
                for p in range(PLAYERS):
                    if ranks[p] == r + 1:
                        line_bufs[r].printf("RANK ", r + 1, ": P", p + 1, " - ", scores[p])
                        shown[r] = 1
                if shown[r] == 1:
                    line_bufs[r].display()
            refreshes += 1

    return {
        "leader": leader,
        "refreshes": refreshes,
    }


def main():
    session = eudkit.Session(BASEMAP_PATH)
    build(session)
    out_path = Path(__file__).parent / "output_v2.scx"
    report = session.compile(out_path)
    if out_path.exists():
        out_path.unlink()
    print("Scenario 69 (v2) compiled successfully:")
    print(f"  Triggers: {report.trigger_count}")
    print(f"  Payload:  {report.payload_size} bytes")
    print(f"  VProcs:   {report.vproc_count}")
    return report


if __name__ == "__main__":
    main()
