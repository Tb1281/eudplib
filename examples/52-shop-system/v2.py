# 52 - shop system (modern eudkit)
#
# Expresses ideal intent:
# 1. First-class `Switch` condition and `SetSwitch` action for beacon event handling.
# 2. Native `Accumulate` and `SetResources` stock primitives with direct branch logic.
# 3. Clean local session state with zero VProcs.
import eudkit


P1 = 0
MARINE = 0
FIREBAT = 10
MEDIC = 34
LOC_BASE = 1

MAX_ATTACK_LEVEL = 3

from eudkit import session

bought = [session.var(0) for _ in range(4)]
attack_level = session.var(0)
ore_spent = session.var(0)
gas_spent = session.var(0)
failures = session.var(0)
last_failed_item = session.var(0)

@session.fn(main_loop=True)
def game_loop() -> None:
    global attack_level, ore_spent, gas_spent, failures, last_failed_item

    # Item 0: Marine pack (sw 0, 100 ore, 4 Marines)
    if eudkit.Switch(0, eudkit.Set):
        eudkit.SetSwitch(0, eudkit.Clear)
        if eudkit.Accumulate(P1, eudkit.AtLeast, 100, eudkit.Ore):
            eudkit.SetResources(P1, eudkit.Subtract, 100, eudkit.Ore)
            ore_spent += 100
            eudkit.CreateUnit(4, MARINE, LOC_BASE, P1)
            bought[0] += 1
        else:
            failures += 1
            last_failed_item = 0
            eudkit.MinimapPing(LOC_BASE)

    # Item 1: Medic pack (sw 1, 75 ore, 2 Medics)
    if eudkit.Switch(1, eudkit.Set):
        eudkit.SetSwitch(1, eudkit.Clear)
        if eudkit.Accumulate(P1, eudkit.AtLeast, 75, eudkit.Ore):
            eudkit.SetResources(P1, eudkit.Subtract, 75, eudkit.Ore)
            ore_spent += 75
            eudkit.CreateUnit(2, MEDIC, LOC_BASE, P1)
            bought[1] += 1
        else:
            failures += 1
            last_failed_item = 1
            eudkit.MinimapPing(LOC_BASE)

    # Item 2: Firebat pack (sw 2, 125 ore, 2 Firebats)
    if eudkit.Switch(2, eudkit.Set):
        eudkit.SetSwitch(2, eudkit.Clear)
        if eudkit.Accumulate(P1, eudkit.AtLeast, 125, eudkit.Ore):
            eudkit.SetResources(P1, eudkit.Subtract, 125, eudkit.Ore)
            ore_spent += 125
            eudkit.CreateUnit(2, FIREBAT, LOC_BASE, P1)
            bought[2] += 1
        else:
            failures += 1
            last_failed_item = 2
            eudkit.MinimapPing(LOC_BASE)

    # Item 3: Attack upgrade (sw 3, 200 ore + 50 gas, max Lv3)
    if eudkit.Switch(3, eudkit.Set):
        eudkit.SetSwitch(3, eudkit.Clear)
        can_buy_upg = 0
        if attack_level < MAX_ATTACK_LEVEL:
            if eudkit.Accumulate(P1, eudkit.AtLeast, 200, eudkit.Ore):
                if eudkit.Accumulate(P1, eudkit.AtLeast, 50, eudkit.Gas):
                    can_buy_upg = 1

        if can_buy_upg == 1:
            eudkit.SetResources(P1, eudkit.Subtract, 200, eudkit.Ore)
            eudkit.SetResources(P1, eudkit.Subtract, 50, eudkit.Gas)
            ore_spent += 200
            gas_spent += 50
            attack_level += 1
            bought[3] += 1
        else:
            failures += 1
            last_failed_item = 3
            eudkit.MinimapPing(LOC_BASE)
