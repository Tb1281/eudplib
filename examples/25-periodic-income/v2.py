"""Scenario 25: Periodic Income and Milestone Economy — Modern eudkit (v2).

Ideal Intent:
Every 150 trigger loops (approx 6.25 seconds on Fastest speed):
1. Pay 50 Ore and 20 Gas to each active player (Players 1 to 4).
2. Every 4th payout is a milestone payout, awarding an extra 100 Ore to each player.
3. Track total payouts, milestones, and lifetime resource disbursements.
Uses declarative reactive timing (`with every(150):`).
"""

import eudkit
from eudkit import (
    session,
    Add,
    Gas,
    Ore,
    SetResources,
    every,
)


INCOME_PERIOD = 150
ORE_PER_PAYOUT = 50
GAS_PER_PAYOUT = 20
MILESTONE_EVERY = 4
MILESTONE_ORE = 100
PLAYERS_COUNT = 4

payouts = session.var(0, name="payouts")
milestone_counter = session.var(0, name="milestone_counter")
milestones = session.var(0, name="milestones")
total_ore_paid = session.var(0, name="total_ore_paid")
total_gas_paid = session.var(0, name="total_gas_paid")
is_milestone = session.var(0, name="is_milestone")

@session.fn
def game_loop():
    # Declarative cyclic timing primitive without manual timer accumulation
    with every(INCOME_PERIOD):
        payouts += 1
        milestone_counter += 1
        is_milestone = 0

        # Milestone evaluation
        if milestone_counter >= MILESTONE_EVERY:
            milestone_counter = 0
            milestones += 1
            is_milestone = 1

        # Build-time unrolled multi-player resource grants
        for p in range(PLAYERS_COUNT):
            SetResources(p, Add, ORE_PER_PAYOUT, Ore)
            SetResources(p, Add, GAS_PER_PAYOUT, Gas)
            if is_milestone == 1:
                SetResources(p, Add, MILESTONE_ORE, Ore)

        # Cumulative bookkeeping
        total_ore_paid += ORE_PER_PAYOUT * PLAYERS_COUNT
        total_gas_paid += GAS_PER_PAYOUT * PLAYERS_COUNT
        if is_milestone == 1:
            total_ore_paid += MILESTONE_ORE * PLAYERS_COUNT
