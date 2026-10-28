# 63 - vote system (legacy eudplib)
#
# Intent: a 4-player map vote. Each player parks their Terran
# Civilian in one of three option zones (A: army, B: wealth,
# C: challenge). The window lasts 360 trigger loops; the civilian's
# current zone IS the vote (moving zones changes it). When the
# window closes, votes are tallied, the majority option wins (ties
# break toward the lower option), and the result is applied to
# everyone: A creates 4 Marines per player at their base, B grants
# 300 ore per player, C spawns a bonus Ultralisk for the enemy.
#
# Counting form (pilot convention): run() builds the scenario's
# triggers once; benchmarks/pilot/count_legacy.py calls it inside a
# fresh PushTriggerScope and reports the GetTriggerCounter() delta.
# Units, locations and players are numeric ids (no basemap in the
# harness); text is a Db string, exactly like the pilot scenarios.
from eudkit import *

CIVILIAN = 15
MARINE = 0
ULTRALISK = 39
ZONE_A = 10
ZONE_B = 11
ZONE_C = 12
BASE0 = 30               # player p's base location is BASE0 + p
CENTER = 40
ENEMY = 7                # Player 8 hosts the challenge spawn
WINDOW = 360             # voting window, in trigger loops
ARMY_PER_PLAYER = 4
WEALTH_ORE = 300

tick = EUDVariable()
votes = [EUDVariable() for _ in range(4)]   # 0 none, 1 A, 2 B, 3 C
cnt_a = EUDVariable()
cnt_b = EUDVariable()
cnt_c = EUDVariable()
votes_cast = EUDVariable()
winner = EUDVariable()                      # 0 unresolved, else 1..3
resolved = EUDVariable()


def sample_votes():
    # The civilian's current zone is the vote; first zone matched
    # wins (the zones are disjoint, so order is only a tie-break).
    for p in range(4):
        player = EncodePlayer(p)
        if EUDIf()(Bring(player, AtLeast, 1, CIVILIAN, ZONE_A)):
            votes[p] << 1
        if EUDElseIf()(Bring(player, AtLeast, 1, CIVILIAN, ZONE_B)):
            votes[p] << 2
        if EUDElseIf()(Bring(player, AtLeast, 1, CIVILIAN, ZONE_C)):
            votes[p] << 3
        EUDEndIf()


def resolve():
    global cnt_a, cnt_b, cnt_c, votes_cast, winner
    # Tally the recorded votes.
    for p in range(4):
        if EUDIf()(votes[p] == 1):
            cnt_a += 1
            votes_cast += 1
        EUDEndIf()
        if EUDIf()(votes[p] == 2):
            cnt_b += 1
            votes_cast += 1
        EUDEndIf()
        if EUDIf()(votes[p] == 3):
            cnt_c += 1
            votes_cast += 1
        EUDEndIf()

    # Winner: most votes, ties toward the lower option.
    if EUDIf()([votes_cast >= 1, cnt_a >= cnt_b, cnt_a >= cnt_c]):
        winner << 1
    EUDEndIf()
    if EUDIf()([votes_cast >= 1, winner == 0,
                cnt_b > cnt_a, cnt_b >= cnt_c]):
        winner << 2
    EUDEndIf()
    if EUDIf()([votes_cast >= 1, winner == 0]):
        winner << 3
    EUDEndIf()

    # Apply the winning option to everyone.
    if EUDIf()(winner == 1):
        for p in range(4):
            DoActions(CreateUnit(ARMY_PER_PLAYER, MARINE,
                                 BASE0 + p, EncodePlayer(p)))
        DoActions(DisplayText(Db("Vote result: ARMY - reinforcements for all.")))
    EUDEndIf()
    if EUDIf()(winner == 2):
        for p in range(4):
            DoActions(SetResources(EncodePlayer(p), Add, WEALTH_ORE, Ore))
        DoActions(DisplayText(Db("Vote result: WEALTH - 300 ore for all.")))
    EUDEndIf()
    if EUDIf()(winner == 3):
        DoActions(CreateUnit(1, ULTRALISK, CENTER, EncodePlayer(ENEMY)))
        DoActions(DisplayText(Db("Vote result: CHALLENGE - good luck.")))
    EUDEndIf()
    if EUDIf()(votes_cast == 0):
        DoActions(DisplayText(Db("Vote closed: no votes were cast.")))
    EUDEndIf()


def run():
    global tick, resolved
    if EUDIf()(resolved == 0):
        tick += 1
        sample_votes()
        if EUDIf()(tick >= WINDOW):
            resolve()
            resolved << 1
        EUDEndIf()
    EUDEndIf()
