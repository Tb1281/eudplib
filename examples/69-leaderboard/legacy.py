# 69 - leaderboard: sort player scores, show top 3 (legacy)
#
# Intent: every 240 trigger loops, rank the 8 players by their
# engine Custom score (the per-player dword array at 0x5822F4)
# and display the top 3 as formatted lines ("RANK 1: P4 - 12500"
# style). Ranking is by counting: rank[p] = 1 + the number of
# players with a strictly higher score (ties share a rank, the
# classic competition ranking). Three reusable DBString buffers
# hold the three lines; each is composed only when some player
# actually holds that rank. Bookkeeping tracks refreshes and the
# current leader (first player found at rank 1).
#
# Counting form (pilot convention): run() builds the scenario's
# triggers once; benchmarks/pilot/count_legacy.py calls it inside
# a fresh PushTriggerScope and reports the GetTriggerCounter()
# delta. DBString.Display() allocates string ids from the loaded
# map, so LoadMap happens at module level (outside the delta),
# exactly like the save-code scenario (G-68). The f_dwread call
# sites ride the import-time readtable engine (80T, session
# amortized, outside the delta) -- the pilot RESULTS asymmetry.
from eudkit import *
from eudkit.string.dbstr import DBString

LoadMap("tests/outputmap/basemap/basemap.scx")

CUSTOM_SCORE_ADDR = 0x5822F4   # + 4 * player (scdata Player.customScore)
REFRESH_PERIOD = 240
PLAYERS = 8

line_bufs = [DBString(48) for _ in range(3)]

scores = [EUDVariable() for _ in range(PLAYERS)]
ranks = [EUDVariable() for _ in range(PLAYERS)]
refresh_tick = EUDVariable()
refreshes = EUDVariable()
leader = EUDVariable()          # rank-1 player index + 1 (0 = unset)
shown = [EUDVariable() for _ in range(3)]   # line r currently shown


def refresh():
    global leader, refreshes
    # Snapshot the engine scores.
    for p in range(PLAYERS):
        scores[p] << f_dwread(CUSTOM_SCORE_ADDR + 4 * p)

    # Competition ranking by pairwise counting.
    for p in range(PLAYERS):
        ranks[p] << 1
        for q in range(PLAYERS):
            if q != p:
                if EUDIf()(scores[q] > scores[p]):
                    ranks[p] += 1
                EUDEndIf()

    # Leader bookkeeping: first player found holding rank 1.
    leader << 0
    for p in range(PLAYERS):
        if EUDIf()([ranks[p] == 1, leader == 0]):
            leader << p + 1
        EUDEndIf()

    # Compose and display the top-3 lines. On a tied rank the
    # last player holding it wins the line (the print restarts
    # the buffer); the ours side mirrors this exactly.
    for r in range(3):
        shown[r] << 0
        for p in range(PLAYERS):
            if EUDIf()(ranks[p] == r + 1):
                f_dbstr_print(line_bufs[r], "RANK ", r + 1,
                              ": P", p + 1, " - ", scores[p])
                shown[r] << 1
            EUDEndIf()
        if EUDIf()(shown[r] == 1):
            line_bufs[r].Display()
        EUDEndIf()
    refreshes += 1


def run():
    global refresh_tick
    refresh_tick += 1
    if EUDIf()(refresh_tick >= REFRESH_PERIOD):
        refresh_tick << 0
        refresh()
    EUDEndIf()
