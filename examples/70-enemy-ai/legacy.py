# 70 - enemy AI director (legacy eudplib)
#
# Intent: a simple strategic AI for the enemy (Player 8). Every
# 90 trigger loops the director takes stock of the enemy Marine
# count and issues ONE standing order set:
#   army >= 12  attack wave: all Marines leave the muster field
#               with an Attack order on the player base.
#   army 4..11  patrol: Marines patrol the midfield (keeps
#               pressure on without committing).
#   army <= 3   retreat: Marines fall back (Move order) to the
#               rally point by the enemy base.
# Independently of the stance, if the army is below the 20 cap
# the barracks reinforce: 2 fresh Marines at the enemy base.
# Bookkeeping counts waves, patrols, retreats and reinforcements.
# (Command reads the engine's cached unit tables -- see the
# edac/142558 caveat in note.md; both sides read it the same way.)
#
# Counting form (pilot convention): run() builds the scenario's
# triggers once; benchmarks/pilot/count_legacy.py calls it inside
# a fresh PushTriggerScope and reports the GetTriggerCounter()
# delta. Units, locations and players are numeric ids (no
# basemap in the harness).
from eudkit import *

MARINE = 0
ENEMY = 7                # Player 8
ENEMY_BASE = 50
MUSTER_FIELD = 52        # where the standing army waits
PLAYER_BASE = 51
MIDFIELD = 54
RALLY = 53
DECISION_PERIOD = 90
ATTACK_THRESHOLD = 12
RETREAT_THRESHOLD = 3
REINFORCE_CAP = 20
REINFORCE_BATCH = 2

ORDER_MOVE = 0           # trigger order ids
ORDER_PATROL = 1
ORDER_ATTACK = 2

decision_tick = EUDVariable()
attack_waves = EUDVariable()
patrol_orders = EUDVariable()
retreats = EUDVariable()
reinforcements = EUDVariable()   # Marines produced by the director
last_stance = EUDVariable()      # 0 none, 1 attack, 2 patrol, 3 retreat


def decide():
    global attack_waves, patrol_orders, retreats, last_stance
    global reinforcements
    enemy = EncodePlayer(ENEMY)

    # Stance selection by army size (one branch per decision).
    if EUDIf()(Command(enemy, AtLeast, ATTACK_THRESHOLD, MARINE)):
        DoActions(Order(MARINE, enemy, MUSTER_FIELD,
                        ORDER_ATTACK, PLAYER_BASE))
        attack_waves += 1
        last_stance << 1
    if EUDElseIf()(Command(enemy, AtLeast, RETREAT_THRESHOLD + 1,
                           MARINE)):
        DoActions(Order(MARINE, enemy, MUSTER_FIELD,
                        ORDER_PATROL, MIDFIELD))
        patrol_orders += 1
        last_stance << 2
    if EUDElse()():
        DoActions(Order(MARINE, enemy, MUSTER_FIELD,
                        ORDER_MOVE, RALLY))
        retreats += 1
        last_stance << 3
    EUDEndIf()

    # Reinforcement runs regardless of stance (cap-checked).
    if EUDIf()(Command(enemy, AtMost, REINFORCE_CAP - 1, MARINE)):
        DoActions(CreateUnit(REINFORCE_BATCH, MARINE,
                             ENEMY_BASE, enemy))
        reinforcements += REINFORCE_BATCH
    EUDEndIf()


def run():
    global decision_tick
    decision_tick += 1
    if EUDIf()(decision_tick >= DECISION_PERIOD):
        decision_tick << 0
        decide()
    EUDEndIf()
