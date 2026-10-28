# 62 - bank: automatic transfer + interest (legacy eudplib)
#
# Intent: a bank system for the four human players (P1..P4),
# settled every 120 loops. For each player, in order:
#   INTEREST: if the player holds at least 1000 ore, pay 5%
#   interest (ore / 20), capped at 200 per settlement, credited
#   directly to the ore account; the payment is booked per player
#   in an interest-paid array. Below 1000 ore, the settlement is
#   counted as a no-interest tick instead.
#   EXCHANGE: if the player then holds at least 500 gas, the bank
#   auto-exchanges 100 gas for 150 ore (the house rate), booked in
#   a lifetime exchange counter.
# Balances live in the engine's per-player resource arrays
# (minerals at 0x57F0F0 + 4*player, gas at 0x57F120 + 4*player).
# Interest is credited with a relative SetDeaths Add on the
# resource EPD (the classic variable-amount credit); the ours side
# uses read-modify-write through its memwrite API instead (see
# note.md).
#
# Counting form (pilot convention): run() builds the scenario's
# triggers once; benchmarks/pilot/count_legacy.py calls it inside a
# fresh PushTriggerScope and reports the GetTriggerCounter() delta.
# In production the body is the beforeTriggerExec hook; run() is
# that body verbatim. Players are numeric ids (no basemap in the
# harness, exactly like the pilot scenarios).
from eudkit import *

PLAYERS = 4             # P1..P4
SETTLE_PERIOD = 120     # loops between settlements
INTEREST_FLOOR = 1000   # minimum ore balance that earns interest
INTEREST_DIVISOR = 20   # 5%
INTEREST_CAP = 200      # max interest per settlement
EXCHANGE_GAS_MIN = 500  # gas balance that triggers an exchange
EXCHANGE_GAS = 100      # gas sold per exchange
EXCHANGE_ORE = 150      # ore bought per exchange

ORE_BASE = 0x57F0F0     # per-player minerals array
GAS_BASE = 0x57F120     # per-player gas array

interest_paid = EUDArray([0] * PLAYERS)  # per-player totals
exchanges_total = EUDVariable(0)  # lifetime exchanges
no_interest_ticks = EUDVariable(0)  # settlements below the floor
bank_ticks = EUDVariable(0)     # settlements run
settle_tick = EUDVariable(0)    # hand-rolled settlement timer
ore_now = EUDVariable(0)        # scratch: current ore balance
gas_now = EUDVariable(0)        # scratch: current gas balance
interest = EUDVariable(0)       # scratch: interest this settlement


def run():
    global exchanges_total, no_interest_ticks, bank_ticks
    global settle_tick, ore_now, gas_now, interest
    settle_tick += 1
    if EUDIf()(settle_tick >= SETTLE_PERIOD):
        settle_tick << 0
        bank_ticks += 1
        for p in range(PLAYERS):
            ore_epd = EPD(ORE_BASE + 4 * p)
            gas_epd = EPD(GAS_BASE + 4 * p)
            ore_now << f_dwread_epd(ore_epd)
            gas_now << f_dwread_epd(gas_epd)

            # Interest: 5% of the ore balance, gated and capped.
            if EUDIf()(ore_now >= INTEREST_FLOOR):
                interest, _rem = f_div(ore_now, INTEREST_DIVISOR)
                if EUDIf()(interest > INTEREST_CAP):
                    interest << INTEREST_CAP
                EUDEndIf()
                # Relative credit on the resource EPD.
                DoActions(SetDeaths(ore_epd, Add, interest, 0))
                interest_paid[p] = interest_paid[p] + interest
                ore_now += interest  # keep the local copy in sync
            EUDEndIf()
            if EUDIf()(ore_now < INTEREST_FLOOR):
                no_interest_ticks += 1
            EUDEndIf()

            # Exchange: sell 100 gas for 150 ore at the house rate.
            if EUDIf()(gas_now >= EXCHANGE_GAS_MIN):
                DoActions([
                    SetDeaths(gas_epd, Subtract, EXCHANGE_GAS, 0),
                    SetDeaths(ore_epd, Add, EXCHANGE_ORE, 0),
                ])
                exchanges_total += 1
            EUDEndIf()
    EUDEndIf()
