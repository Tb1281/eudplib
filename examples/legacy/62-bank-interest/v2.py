# 62 - bank: automatic transfer + interest (modern eudkit)
#
# Expresses ideal intent:
# 1. Periodic settlement via `with session.every(120):`.
# 2. Per-player interest calculation with floor check and cap limiting.
# 3. Direct resource exchange logic and array bookkeeping with zero global singletons.
import eudkit


PLAYERS = 4
SETTLE_PERIOD = 120
INTEREST_FLOOR = 1000
INTEREST_DIVISOR = 20
INTEREST_CAP = 200
EXCHANGE_GAS_MIN = 500
EXCHANGE_GAS = 100
EXCHANGE_ORE = 150

ORE_BASE = 0x57F0F0
GAS_BASE = 0x57F120

from eudkit import session

interest_paid = eudkit.Array(PLAYERS)
exchanges_total = session.var(0)
no_interest_ticks = session.var(0)
bank_ticks = session.var(0)

@session.fn(main_loop=True)
def game_loop() -> None:
    global exchanges_total, no_interest_ticks, bank_ticks

    with session.every(SETTLE_PERIOD):
        bank_ticks += 1
        for p in range(PLAYERS):
            ore_addr = ORE_BASE + 4 * p
            gas_addr = GAS_BASE + 4 * p
            ore_now = eudkit.dwread(ore_addr)
            gas_now = eudkit.dwread(gas_addr)

            if ore_now >= INTEREST_FLOOR:
                interest = ore_now // INTEREST_DIVISOR
                if interest > INTEREST_CAP:
                    interest = INTEREST_CAP
                ore_now += interest
                eudkit.dwwrite(ore_addr, ore_now)
                interest_paid[p] = interest_paid[p] + interest
            else:
                no_interest_ticks += 1

            if gas_now >= EXCHANGE_GAS_MIN:
                eudkit.dwwrite(gas_addr, gas_now - EXCHANGE_GAS)
                eudkit.dwwrite(ore_addr, ore_now + EXCHANGE_ORE)
                exchanges_total += 1
