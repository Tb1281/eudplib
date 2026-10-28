# 70 - enemy AI director (modern eudkit)
#
# Expresses ideal intent:
# 1. Periodic AI decisions using native `with session.every(90):` clock rule.
# 2. Army threshold evaluation and order issuance using stock `Order` actions (Attack, Patrol, Move).
# 3. Automatic reinforcement production with unit cap checks.
import eudkit


MARINE = 0
ENEMY = 7
ENEMY_BASE = 50
MUSTER_FIELD = 52
PLAYER_BASE = 51
MIDFIELD = 54
RALLY = 53
DECISION_PERIOD = 90
ATTACK_THRESHOLD = 12
RETREAT_THRESHOLD = 3
REINFORCE_CAP = 20
REINFORCE_BATCH = 2

from eudkit import session

attack_waves = session.var(0)
patrol_orders = session.var(0)
retreats = session.var(0)
reinforcements = session.var(0)
last_stance = session.var(0)

@session.fn(main_loop=True)
def game_loop() -> None:
    global attack_waves, patrol_orders, retreats, reinforcements, last_stance

    with session.every(DECISION_PERIOD):
        if eudkit.Command(ENEMY, eudkit.AtLeast, ATTACK_THRESHOLD, MARINE):
            eudkit.Order(MARINE, ENEMY, MUSTER_FIELD, eudkit.Attack, PLAYER_BASE)
            attack_waves += 1
            last_stance = 1
        elif eudkit.Command(ENEMY, eudkit.AtLeast, RETREAT_THRESHOLD + 1, MARINE):
            eudkit.Order(MARINE, ENEMY, MUSTER_FIELD, eudkit.Patrol, MIDFIELD)
            patrol_orders += 1
            last_stance = 2
        else:
            eudkit.Order(MARINE, ENEMY, MUSTER_FIELD, eudkit.Move, RALLY)
            retreats += 1
            last_stance = 3

        if eudkit.Command(ENEMY, eudkit.AtMost, REINFORCE_CAP - 1, MARINE):
            eudkit.CreateUnit(REINFORCE_BATCH, MARINE, ENEMY_BASE, ENEMY)
            reinforcements += REINFORCE_BATCH
