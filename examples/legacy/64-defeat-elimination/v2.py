# 64 - defeat / elimination handling (modern eudkit)
#
# Expresses ideal intent:
# 1. First-appearance detection for active players via Command Center checks.
# 2. Complete board cleanup via stock `eudkit.KillUnit(ANY_UNIT, p)` action.
# 3. Last-man-standing victory detection and string announcements.
import eudkit


COMMAND_CENTER = 106
ANY_UNIT = 229

from eudkit import session

started = [session.var(0) for _ in range(4)]
out = [session.var(0) for _ in range(4)]
alive_count = session.var(0)
winner = session.var(0)
game_over = session.var(0)

msg_eliminated = session.map_string("A command center has fallen - a player is eliminated.")
msg_victory = session.map_string("One player remains - victory!")

@session.fn(main_loop=True)
def game_loop() -> None:
    global alive_count, winner, game_over

    for p in range(4):
        if started[p] == 0 and eudkit.Command(p, eudkit.AtLeast, 1, COMMAND_CENTER):
            started[p] = 1
            alive_count += 1

        if started[p] == 1 and out[p] == 0 and eudkit.Command(p, eudkit.Exactly, 0, COMMAND_CENTER):
            out[p] = 1
            alive_count -= 1
            eudkit.KillUnit(ANY_UNIT, p)
            msg_eliminated.display()

    if game_over == 0 and alive_count == 1:
        for p in range(4):
            if started[p] == 1 and out[p] == 0:
                winner = p + 1
        msg_victory.display()
        game_over = 1
