# 52 - shop system (legacy eudplib)
#
# Intent: a beacon shop for Player 1, driven by four request
# switches (set by classic map triggers when the player steps on a
# beacon; this scenario is the purchase processor). Each loop, a
# set switch is consumed exactly once:
#   SW 0: Marine pack   -- 100 ore        -> 4 Marines at base
#   SW 1: Medic pack    --  75 ore        -> 2 Medics at base
#   SW 2: Firebat pack  -- 125 ore        -> 2 Firebats at base
#   SW 3: Attack upgrade-- 200 ore + 50 gas -> attack level +1 (max 3)
# A purchase checks the balance with the Accumulate condition,
# charges with SetResources, delivers, and latches bookkeeping
# (per-item counts, ore/gas spent). A failed check (not enough
# resources, or the upgrade already maxed) clears the switch too,
# pings the base, and records the failure with the item id.
#
# Counting form (pilot convention): run() builds the scenario's
# triggers once; benchmarks/pilot/count_legacy.py calls it inside a
# fresh PushTriggerScope and reports the GetTriggerCounter() delta.
# In production the body is the beforeTriggerExec hook; run() is
# that body verbatim. Units, locations and switches are numeric ids
# (no basemap in the harness, exactly like the pilot scenarios); a
# real map build resolves the same calls by name and creates the
# identical triggers. The menu table is expanded at build time --
# the standard legacy codegen style for a fixed shop menu.
from eudkit import *

P1 = 0
MARINE = 0          # unit ids
FIREBAT = 10
MEDIC = 34
LOC_BASE = 1        # location id

MAX_ATTACK_LEVEL = 3

# Menu rows: (switch, ore price, gas price, unit, count).
# unit -1 = the upgrade row (no unit delivered).
MENU = [
    (0, 100, 0, MARINE, 4),
    (1, 75, 0, MEDIC, 2),
    (2, 125, 0, FIREBAT, 2),
    (3, 200, 50, -1, 0),
]

bought = [EUDVariable(0) for _ in range(4)]  # per-item counts
attack_level = EUDVariable(0)   # upgrade level 0..3
ore_spent = EUDVariable(0)      # lifetime ore spent
gas_spent = EUDVariable(0)      # lifetime gas spent
failures = EUDVariable(0)       # rejected purchase attempts
last_failed_item = EUDVariable(0)  # item id of the last rejection
can_afford = EUDVariable(0)     # scratch: affordability flag


def run():
    global attack_level, ore_spent, gas_spent, failures
    global last_failed_item, can_afford
    for idx, (sw, ore_price, gas_price, unit, count) in enumerate(MENU):
        if EUDIf()(Switch(sw, Set)):
            # Consume the request exactly once, success or failure.
            DoActions(SetSwitch(sw, Clear))

            # Affordability flag: start optimistic, let each failed
            # check veto. (Compound resource conditions cannot be
            # negated as one EUDIf, so the flag pattern is the
            # standard legacy shape -- the ours side builds the same
            # flag as a compound Cond.)
            can_afford << 1
            if EUDIf()(Accumulate(P1, AtMost, ore_price - 1, Ore)):
                can_afford << 0
            EUDEndIf()
            if gas_price > 0:
                if EUDIf()(Accumulate(P1, AtMost, gas_price - 1, Gas)):
                    can_afford << 0
                EUDEndIf()
            if idx == 3:
                if EUDIf()(attack_level >= MAX_ATTACK_LEVEL):
                    can_afford << 0
                EUDEndIf()

            if EUDIf()(can_afford == 1):
                DoActions(SetResources(P1, Subtract, ore_price, Ore))
                if gas_price > 0:
                    DoActions(SetResources(P1, Subtract, gas_price, Gas))
                ore_spent += ore_price
                gas_spent += gas_price
                if unit >= 0:
                    DoActions(CreateUnit(count, unit, LOC_BASE, P1))
                else:
                    attack_level += 1
                bought[idx] += 1
            EUDEndIf()

            if EUDIf()(can_afford == 0):
                failures += 1
                last_failed_item << idx
                DoActions(MinimapPing(LOC_BASE))
            EUDEndIf()
        EUDEndIf()
