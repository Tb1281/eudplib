# 59 - hero revive (modern eudkit)
#
# Expresses ideal intent:
# 1. State machine for hero death detection and progressive revive delays.
# 2. Map strings for zero-overhead static announcements.
# 3. Clean local session variables and declarative action lists.
import eudkit


P1 = 0
MARINE = 0
LOC_BASE = 1

BASE_WAIT = 180
WAIT_PER_DEATH = 30
REVIVE_COST = 100

from eudkit import session

hero_state = session.var(0)     # 0 = alive, 1 = dead
countdown = session.var(0)
waiting = session.var(0)
deaths = session.var(0)
revives = session.var(0)
total_dead_loops = session.var(0)
total_fees = session.var(0)

msg_fallen = session.map_string("Hero has fallen")
msg_returned = session.map_string("Hero has returned")

@session.fn(main_loop=True)
def game_loop() -> None:
    global hero_state, countdown, waiting, deaths, revives
    global total_dead_loops, total_fees

    if hero_state == 0:
        if eudkit.Command(P1, eudkit.Exactly, 0, MARINE):
            hero_state = 1
            deaths += 1
            countdown = BASE_WAIT + deaths * WAIT_PER_DEATH
            eudkit.MinimapPing(LOC_BASE)
            msg_fallen.display()

    if hero_state == 1:
        total_dead_loops += 1
        if countdown >= 1:
            countdown -= 1

        if countdown == 0:
            if eudkit.Accumulate(P1, eudkit.AtLeast, REVIVE_COST, eudkit.Ore):
                eudkit.SetResources(P1, eudkit.Subtract, REVIVE_COST, eudkit.Ore)
                eudkit.CreateUnit(1, MARINE, LOC_BASE, P1)
                eudkit.MinimapPing(LOC_BASE)
                msg_returned.display()
                total_fees += REVIVE_COST
                hero_state = 0
                waiting = 0
                revives += 1
            else:
                waiting = 1
