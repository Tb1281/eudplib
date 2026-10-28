"""LEGACY kill bounty + broadcast announcement (benchmark 29).

Same intent as ours.js: watch Player 1's Marine deaths counter.
When it increases, the delta is paid as a bounty (15 ore per kill,
plus a flat 50-ore jackpot bonus when 5+ kills land in one loop)
and the whole server sees an announcement composed into a DBString
buffer and displayed. The counter VALUE is needed (delta math and
the announcement), so it is materialized with f_dwread_epd --
legacy's amortized read call site. Bookkeeping: kills paid,
bounties paid, jackpots, announcements shown.
"""

from eudkit import *
from eudkit.string.dbstr import DBString

# DBString.Display() allocates a string id from the loaded map
# (as in any real legacy build, which always LoadMaps first).
# Module level, so it is outside the counted run() delta.
LoadMap("tests/outputmap/basemap/basemap.scx")

# Deaths table address for (Player1, Terran Marine):
# 0x58A364 + 4*player + 48*unit, player 0, unit 0.
MARINE_DEATHS_P1 = 0x58A364
ORE_ADDR_P1 = 0x57F0F0   # Player 1 ore dword
BOUNTY_PER_KILL = 15
JACKPOT_MIN_KILLS = 5    # kills in one loop that trigger a jackpot
JACKPOT_BONUS = 50

announce_buf = DBString(128)  # the one reusable announcement buffer
prev_kills = EUDVariable()     # deaths counter at the last check
cur_kills = EUDVariable()      # deaths counter now (materialized)
new_kills = EUDVariable()      # kills landed since the last check
bounty = EUDVariable()         # ore to pay for this batch
total_kills_paid = EUDVariable()  # lifetime kills rewarded
total_bounty = EUDVariable()   # lifetime ore paid
jackpots = EUDVariable()       # jackpot batches so far
announcements = EUDVariable()  # announcements shown


def run():
    global prev_kills, cur_kills, new_kills, bounty, total_kills_paid
    global total_bounty, jackpots, announcements
    cur_kills << f_dwread_epd(EPD(MARINE_DEATHS_P1))

    if EUDIf()(cur_kills > prev_kills):
        new_kills << cur_kills - prev_kills
        bounty << new_kills * BOUNTY_PER_KILL

        # Jackpot: a big batch pays a flat bonus on top.
        if EUDIf()(new_kills >= JACKPOT_MIN_KILLS):
            bounty += JACKPOT_BONUS
            jackpots += 1
        EUDEndIf()

        # Pay: SetDeaths on the ore dword's EPD, variable amount
        # (the action's number field is patched with the var value).
        DoActions(SetDeaths(EPD(ORE_ADDR_P1), Add, bounty, 0))
        total_kills_paid += new_kills
        total_bounty += bounty

        # Announce: compose into the single buffer, then display.
        f_dbstr_print(announce_buf, "Player 1 scored ", new_kills,
                      " kills! Bounty: ", bounty, " ore")
        announce_buf.Display()
        announcements += 1
    EUDEndIf()

    prev_kills << cur_kills
