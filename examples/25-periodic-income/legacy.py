"""LEGACY periodic income (benchmark 25).

Same intent as ours.js: every 150 trigger loops, each of the
scenario's four players receives 50 ore and 20 gas (the classic
hand-rolled timer-variable pattern). Every 4th payout is a milestone payout and
adds a 100-ore bonus per player. Bookkeeping: payout count, total
ore and gas granted, milestone count.
"""

from eudkit import *

INCOME_PERIOD = 150      # loops between payouts
ORE_PER_PAYOUT = 50
GAS_PER_PAYOUT = 20
MILESTONE_EVERY = 4      # payouts per milestone
MILESTONE_ORE = 100
PLAYERS_IN_GAME = 4      # P1..P4 receive income in this scenario

income_timer = EUDVariable()   # loops since the last payout
payouts = EUDVariable()        # payouts made so far
milestone_counter = EUDVariable()  # payouts since the last milestone
milestones = EUDVariable()     # milestone payouts made
total_ore_paid = EUDVariable()   # lifetime ore granted (all players)
total_gas_paid = EUDVariable()   # lifetime gas granted (all players)
is_milestone = EUDVariable()   # 1 on milestone payouts (scratch flag)


def run():
    global income_timer, payouts, milestone_counter, milestones
    global total_ore_paid, total_gas_paid, is_milestone
    # Hand-rolled timer: count loops, fire at the period, reset.
    income_timer += 1
    if EUDIf()(income_timer >= INCOME_PERIOD):
        income_timer << 0
        payouts += 1
        milestone_counter += 1

        # Milestone check first: the bonus rides the same payout.
        is_milestone << 0
        if EUDIf()(milestone_counter >= MILESTONE_EVERY):
            milestone_counter << 0
            milestones += 1
            is_milestone << 1
        EUDEndIf()

        # Per-player grant, unrolled at build time over the
        # scenario's four players (see ours.js header for why the
        # pair avoids EUDLoopPlayer here).
        for player in (Player1, Player2, Player3, Player4):
            DoActions([
                SetResources(player, Add, ORE_PER_PAYOUT, Ore),
                SetResources(player, Add, GAS_PER_PAYOUT, Gas),
            ])
            if EUDIf()(is_milestone == 1):
                DoActions(SetResources(player, Add, MILESTONE_ORE, Ore))
            EUDEndIf()

        # Bookkeeping assumes the scenario's 4 active players.
        total_ore_paid += ORE_PER_PAYOUT * PLAYERS_IN_GAME
        total_gas_paid += GAS_PER_PAYOUT * PLAYERS_IN_GAME
        if EUDIf()(is_milestone == 1):
            total_ore_paid += MILESTONE_ORE * PLAYERS_IN_GAME
        EUDEndIf()
    EUDEndIf()
