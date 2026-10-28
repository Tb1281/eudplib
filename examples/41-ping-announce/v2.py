# 41 - minimap ping + warning with cooldown (modern eudkit)
#
# Expresses ideal intent:
# 1. Native `Bring` condition and `MinimapPing` stock trigger action.
# 2. Local countdown timers managed without global singletons.
# 3. Clean local session state with zero VProcs.
import eudkit


COOLDOWN_FRAMES = 168   # ~10 s at Fastest (16.7 trigger loops / s)
ENEMY = eudkit.P2

MARINE = 0
HYDRALISK = 38
ZERGLING = 37

WATCH_ZONE = 1
GATE_ZONE = 2
REAR_ZONE = 3

from eudkit import session

watch_cd = session.var(0)
gate_cd = session.var(0)
rear_cd = session.var(0)

@session.fn(main_loop=True)
def game_loop() -> None:
    global watch_cd, gate_cd, rear_cd
    if watch_cd >= 1:
        watch_cd -= 1
    if gate_cd >= 1:
        gate_cd -= 1
    if rear_cd >= 1:
        rear_cd -= 1

    if eudkit.Bring(ENEMY, eudkit.AtLeast, 1, MARINE, WATCH_ZONE):
        if watch_cd == 0:
            watch_cd = COOLDOWN_FRAMES
            eudkit.MinimapPing(WATCH_ZONE)
            eudkit.DisplayText("Warning: enemy Marine in the Watch Zone!")

    if eudkit.Bring(ENEMY, eudkit.AtLeast, 1, HYDRALISK, GATE_ZONE):
        if gate_cd == 0:
            gate_cd = COOLDOWN_FRAMES
            eudkit.MinimapPing(GATE_ZONE)
            eudkit.DisplayText("Warning: enemy Hydralisk at the Gate Zone!")

    if eudkit.Bring(ENEMY, eudkit.AtLeast, 1, ZERGLING, REAR_ZONE):
        if rear_cd == 0:
            rear_cd = COOLDOWN_FRAMES
            eudkit.MinimapPing(REAR_ZONE)
            eudkit.DisplayText("Warning: enemy Zergling in the Rear Zone!")
