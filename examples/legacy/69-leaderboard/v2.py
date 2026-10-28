# 69 - leaderboard: sort player scores, show top 3 (modern eudkit)
#
# Expresses ideal intent:
# 1. Consolidated `StringBuffer` API for single dynamic string architecture (ADR 0004).
# 2. Declarative refresh interval via `with session.every(240):`.
# 3. Competition ranking with pairwise score comparison and formatted display.
import eudkit


CUSTOM_SCORE_ADDR = 0x5822F4
REFRESH_PERIOD = 240
PLAYERS = 8

from eudkit import session

line_bufs = [session.string(48) for _ in range(3)]

scores = [session.var(0) for _ in range(PLAYERS)]
ranks = [session.var(0) for _ in range(PLAYERS)]
refreshes = session.var(0)
leader = session.var(0)
shown = [session.var(0) for _ in range(3)]

@session.fn(main_loop=True)
def game_loop() -> None:
    global leader, refreshes

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
