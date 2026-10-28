# 64 - defeat / elimination handling (legacy eudplib)
#
# Intent: 4-player free-for-all. A player is IN the game once their
# Command Center has been seen (the started flag -- a player who
# never had one is not "eliminated", they were never playing).
# When a started player's Command Centers reach zero, they are
# eliminated: an announcement, every remaining unit of theirs is
# killed (the classic KillUnit "Any unit" cleanup), and the alive
# count drops. When exactly one started, non-eliminated player
# remains, the game is decided: the survivor is recorded as the
# winner and announced once.
#
# Counting form (pilot convention): run() builds the scenario's
# triggers once; benchmarks/pilot/count_legacy.py calls it inside a
# fresh PushTriggerScope and reports the GetTriggerCounter() delta.
# Units and players are numeric ids (no basemap in the harness);
# text is a Db string, as in the pilot scenarios.
from eudkit import *

COMMAND_CENTER = 106
ANY_UNIT = 229           # trigger unit-list pseudo id "Any Unit"

started = [EUDVariable() for _ in range(4)]   # CC seen at least once
out = [EUDVariable() for _ in range(4)]       # elimination processed
alive_count = EUDVariable()                   # started and not out
winner = EUDVariable()                        # survivor's index + 1
game_over = EUDVariable()


def watch_player(p):
    global alive_count
    player = EncodePlayer(p)
    # Arm the watcher: the player counts as playing once a Command
    # Center exists (checked every loop until it first appears).
    if EUDIf()([started[p] == 0,
                Command(player, AtLeast, 1, COMMAND_CENTER)]):
        started[p] << 1
        alive_count += 1
    EUDEndIf()

    # Elimination edge: was in the game, CC count just hit zero.
    if EUDIf()([started[p] == 1, out[p] == 0,
                Command(player, Exactly, 0, COMMAND_CENTER)]):
        out[p] << 1
        alive_count -= 1
        # Cleanup: remove the eliminated player's whole board.
        DoActions(KillUnit(ANY_UNIT, player))
        DoActions(DisplayText(
            Db("A command center has fallen - a player is eliminated.")))
    EUDEndIf()


def check_victory():
    global winner, game_over
    if EUDIf()([game_over == 0, alive_count == 1]):
        # The survivor is the one started player who is not out.
        for p in range(4):
            if EUDIf()([started[p] == 1, out[p] == 0]):
                winner << p + 1
            EUDEndIf()
        DoActions(DisplayText(Db("One player remains - victory!")))
        game_over << 1
    EUDEndIf()


def run():
    for p in range(4):
        watch_player(p)
    check_victory()
