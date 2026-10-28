# 60 - skill: mana, cooldown, area damage (modern eudkit)
#
# Expresses ideal intent:
# 1. State machine managing mana recharge rates (ready vs cooling).
# 2. Native python conditionals for casting conditions and failed cast accounting.
# 3. Clean local session variables and declarative action lists.
import eudkit


P8 = 7
ZERGLING = 37
HYDRALISK = 38
LOC_NOVA = 5
LOC_HERO = 6

MANA_MAX = 100
REGEN_READY = 3
REGEN_COOLING = 1
NOVA_COOLDOWN = 240
NOVA_ZERGLINGS = 4
NOVA_HYDRALISKS = 2
SW_NOVA = 8

from eudkit import session

mana = session.var(0)
cooldown = session.var(0)
ready_flag = session.var(0)
casts = session.var(0)
failed_casts = session.var(0)
total_mana_spent = session.var(0)

@session.fn(main_loop=True)
def game_loop() -> None:
    global mana, cooldown, ready_flag, casts, failed_casts, total_mana_spent

    if cooldown >= 1:
        cooldown -= 1

    if cooldown == 0:
        if mana <= MANA_MAX - REGEN_READY:
            mana += REGEN_READY
        elif mana < MANA_MAX:
            mana = MANA_MAX
    else:
        if mana < MANA_MAX:
            mana += REGEN_COOLING

    if eudkit.Switch(SW_NOVA, eudkit.Set):
        eudkit.SetSwitch(SW_NOVA, eudkit.Clear)
        if mana == MANA_MAX and cooldown == 0:
            eudkit.KillUnitAt(NOVA_ZERGLINGS, ZERGLING, LOC_NOVA, P8)
            eudkit.KillUnitAt(NOVA_HYDRALISKS, HYDRALISK, LOC_NOVA, P8)
            eudkit.MinimapPing(LOC_NOVA)
            mana = 0
            cooldown = NOVA_COOLDOWN
            ready_flag = 0
            casts += 1
            total_mana_spent += MANA_MAX
        else:
            failed_casts += 1

    if cooldown == 0 and mana == MANA_MAX:
        if ready_flag == 0:
            eudkit.MinimapPing(LOC_HERO)
            ready_flag = 1
