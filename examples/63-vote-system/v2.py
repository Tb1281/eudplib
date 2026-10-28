# 63 - vote system (modern eudkit)
#
# Expresses ideal intent:
# 1. Timed voting window with zone polling per player.
# 2. Majority tally and tie-breaking resolution.
# 3. Deduplicated map strings for event broadcasts.
import eudkit


CIVILIAN = 15
MARINE = 0
ULTRALISK = 39
ZONE_A = 10
ZONE_B = 11
ZONE_C = 12
BASE0 = 30
CENTER = 40
ENEMY = 7
WINDOW = 360
ARMY_PER_PLAYER = 4
WEALTH_ORE = 300

from eudkit import session

tick = session.var(0)
votes = [session.var(0) for _ in range(4)]
cnt_a = session.var(0)
cnt_b = session.var(0)
cnt_c = session.var(0)
votes_cast = session.var(0)
winner = session.var(0)
resolved = session.var(0)

msg_army = session.map_string("Vote result: ARMY - reinforcements for all.")
msg_wealth = session.map_string("Vote result: WEALTH - 300 ore for all.")
msg_challenge = session.map_string("Vote result: CHALLENGE - good luck.")
msg_none = session.map_string("Vote closed: no votes were cast.")

@session.fn(main_loop=True)
def game_loop() -> None:
    global tick, cnt_a, cnt_b, cnt_c, votes_cast, winner, resolved

    if resolved == 0:
        tick += 1
        for p in range(4):
            if eudkit.Bring(p, eudkit.AtLeast, 1, CIVILIAN, ZONE_A):
                votes[p] = 1
            elif eudkit.Bring(p, eudkit.AtLeast, 1, CIVILIAN, ZONE_B):
                votes[p] = 2
            elif eudkit.Bring(p, eudkit.AtLeast, 1, CIVILIAN, ZONE_C):
                votes[p] = 3

        if tick >= WINDOW:
            for p in range(4):
                if votes[p] == 1:
                    cnt_a += 1
                    votes_cast += 1
                elif votes[p] == 2:
                    cnt_b += 1
                    votes_cast += 1
                elif votes[p] == 3:
                    cnt_c += 1
                    votes_cast += 1

            if votes_cast >= 1 and cnt_a >= cnt_b and cnt_a >= cnt_c:
                winner = 1
            elif votes_cast >= 1 and cnt_b > cnt_a and cnt_b >= cnt_c:
                winner = 2
            elif votes_cast >= 1:
                winner = 3

            if winner == 1:
                for p in range(4):
                    eudkit.CreateUnit(ARMY_PER_PLAYER, MARINE, BASE0 + p, p)
                msg_army.display()
            elif winner == 2:
                for p in range(4):
                    eudkit.SetResources(p, eudkit.Add, WEALTH_ORE, eudkit.Ore)
                msg_wealth.display()
            elif winner == 3:
                eudkit.CreateUnit(1, ULTRALISK, CENTER, ENEMY)
                msg_challenge.display()
            elif votes_cast == 0:
                msg_none.display()

            resolved = 1
