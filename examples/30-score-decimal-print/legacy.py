"""LEGACY decimal scoreboard print (benchmark 30).

Same intent as ours.js: track Player 1's score from two deaths
counters (Zergling deaths of Player 8 = kills scored, Marine
deaths of Player 1 = deaths suffered), and every 120 trigger
loops print one formatted decimal line with score, kills, deaths
and accuracy into a reusable DBString buffer and display it.
Score = kills*100 - deaths*30, floored at 0; accuracy =
kills*100/(kills+deaths), defined as 0 before the first exchange.
f_dbstr_print renders numbers in decimal by default.
"""

from eudkit import *
from eudkit.string.dbstr import DBString

# DBString.Display() allocates a string id from the loaded map
# (as in any real legacy build, which always LoadMaps first).
# Module level, so it is outside the counted run() delta.
LoadMap("tests/outputmap/basemap/basemap.scx")

# Deaths table addresses: 0x58A364 + 4*player + 48*unit.
KILLS_ADDR = 0x58A364 + 4 * 7 + 48 * 37    # Player 8, Zergling
DEATHS_ADDR = 0x58A364                     # Player 1, Marine
PRINT_PERIOD = 120       # loops between scoreboard refreshes
SCORE_PER_KILL = 100
PENALTY_PER_DEATH = 30

board_buf = DBString(128)  # the one reusable scoreboard buffer
print_timer = EUDVariable()   # loops since the last refresh
prev_kills = EUDVariable()
prev_deaths = EUDVariable()
cur = EUDVariable()           # scratch: counter snapshot
kills_total = EUDVariable()
deaths_total = EUDVariable()
score_base = EUDVariable()    # kills * SCORE_PER_KILL (scratch)
score_penalty = EUDVariable()  # deaths * PENALTY_PER_DEATH (scratch)
score = EUDVariable()
accuracy = EUDVariable()
acc_num = EUDVariable()       # scratch: accuracy numerator
acc_den = EUDVariable()       # scratch: accuracy denominator
prints = EUDVariable()        # scoreboard refreshes so far


def accumulate(addr, total_var, prev_var):
    """Fold one deaths counter's delta into its total."""
    global cur
    cur << f_dwread_epd(EPD(addr))
    if EUDIf()(cur > prev_var):
        total_var += cur - prev_var
    EUDEndIf()
    prev_var << cur


def run():
    global print_timer, kills_total, deaths_total, score_base
    global score_penalty, score, accuracy, prints, acc_num, acc_den
    accumulate(KILLS_ADDR, kills_total, prev_kills)
    accumulate(DEATHS_ADDR, deaths_total, prev_deaths)

    # Score with a floor at 0 (unsigned vars cannot go negative).
    score_base << kills_total * SCORE_PER_KILL
    score_penalty << deaths_total * PENALTY_PER_DEATH
    if EUDIf()(score_base >= score_penalty):
        score << score_base - score_penalty
    if EUDElse()():
        score << 0
    EUDEndIf()

    # Periodic refresh, hand-rolled timer like all legacy loops.
    print_timer += 1
    if EUDIf()(print_timer >= PRINT_PERIOD):
        print_timer << 0
        # Accuracy with a divide-by-zero guard. (Legacy has no
        # var/var `/` operator; f_div is the EUDFunc form.)
        if EUDIf()(kills_total + deaths_total >= 1):
            acc_num << kills_total * 100
            acc_den << kills_total + deaths_total
            accuracy << f_div(acc_num, acc_den)[0]  # f_div returns (quot, rem)
        if EUDElse()():
            accuracy << 0
        EUDEndIf()
        f_dbstr_print(board_buf, "SCORE: ", score, "  KILLS: ",
                      kills_total, "  DEATHS: ", deaths_total,
                      "  ACC: ", accuracy, "%")
        board_buf.Display()
        prints += 1
    EUDEndIf()
