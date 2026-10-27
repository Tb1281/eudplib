"""LEGACY random lottery (benchmark 32).

Same intent as ours.js: every 200 trigger loops, each of the
scenario's four players draws one lottery roll (0..99 from the game RNG via the
f_dwrand EUDFunc). Prize tiers: roll < 5 pays a 500-ore jackpot,
5..24 pays 100 ore, 25..59 pays 25 ore, 60+ pays nothing.
Bookkeeping: draws, jackpots, total ore paid, best (lowest) roll
seen. The draw loop is the hand-rolled timer pattern with the
per-player draw unrolled at build time over the scenario's four
players; branching is an if/elif chain on the roll.
"""

from eudkit import *

LOTTERY_PERIOD = 200     # loops between draws
JACKPOT_ROLL = 5         # roll < 5: jackpot
BIG_ROLL = 25            # roll < 25: big prize
SMALL_ROLL = 60          # roll < 60: small prize
JACKPOT_ORE = 500
BIG_ORE = 100
SMALL_ORE = 25

lottery_timer = EUDVariable()  # loops since the last draw
draws = EUDVariable()          # total draws made (all players)
jackpots = EUDVariable()       # jackpot draws so far
total_paid = EUDVariable()     # lifetime ore paid out
best_roll = EUDVariable()      # lowest roll seen (set on first draw)
seen_any = EUDVariable()       # 1 once any draw has been made
roll = EUDVariable()           # scratch: the current draw's roll


def run():
    global lottery_timer, draws, jackpots, total_paid, best_roll, roll
    global seen_any
    lottery_timer += 1
    if EUDIf()(lottery_timer >= LOTTERY_PERIOD):
        lottery_timer << 0

        # 4-player map: the draw is unrolled per player at build
        # time (see ours.js header for why the pair avoids
        # EUDLoopPlayer here).
        for player in (Player1, Player2, Player3, Player4):
            # One draw: game RNG dword, reduced to 0..99.
            roll << f_dwrand() % 100
            draws += 1

            # Prize branch chain (mutually exclusive tiers).
            if EUDIf()(roll < JACKPOT_ROLL):
                DoActions(SetResources(player, Add, JACKPOT_ORE, Ore))
                total_paid += JACKPOT_ORE
                jackpots += 1
            if EUDElseIf()(roll < BIG_ROLL):
                DoActions(SetResources(player, Add, BIG_ORE, Ore))
                total_paid += BIG_ORE
            if EUDElseIf()(roll < SMALL_ROLL):
                DoActions(SetResources(player, Add, SMALL_ORE, Ore))
                total_paid += SMALL_ORE
            EUDEndIf()

            # Best-roll tracking: the first draw initializes it.
            if EUDIf()(seen_any == 0):
                best_roll << roll
                seen_any << 1
            EUDEndIf()
            if EUDIf()(roll < best_roll):
                best_roll << roll
            EUDEndIf()
    EUDEndIf()
