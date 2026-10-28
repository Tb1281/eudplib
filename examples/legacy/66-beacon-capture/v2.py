# 66 - beacon capture (modern eudkit)
#
# Expresses ideal intent:
# 1. First-arrival capture latch with outpost ownership transfer via `eudkit.GiveUnits`.
# 2. Player color palette mutation in engine memory via `bwrite`.
# 3. Deduplicated map strings and clean session state encapsulation.
import eudkit


MARINE = 0
SCV = 7
SIEGE_TANK = 5
OUTPOST = 20
BEACON = 21
NEUTRAL = 11
COLOR_ADDR = 0x581D76
GOLD = 0x9C
SALVAGE_ORE = 50

from eudkit import session

captured = session.var(0)
capturer = session.var(0)
capture_loop = session.var(0)
loop_count = session.var(0)
relay_announced = session.var(0)

msg_captured = session.map_string("The outpost has been captured! Its units change sides.")
msg_relay = session.map_string("The gold outpost still flies its capturer's colors.")

@session.fn(main_loop=True)
def game_loop() -> None:
    global captured, capturer, capture_loop, loop_count, relay_announced

    loop_count += 1
    for p in range(4):
        if captured == 0 and eudkit.Bring(p, eudkit.AtLeast, 1, MARINE, BEACON):
            captured = 1
            capturer = p
            capture_loop = loop_count
            eudkit.GiveUnits(eudkit.All, SCV, NEUTRAL, OUTPOST, p)
            eudkit.GiveUnits(eudkit.All, SIEGE_TANK, NEUTRAL, OUTPOST, p)
            eudkit.bwrite(COLOR_ADDR + p, GOLD)
            eudkit.SetResources(p, eudkit.Add, SALVAGE_ORE, eudkit.Ore)
            msg_captured.display()

    if captured == 1 and relay_announced == 0 and loop_count >= 2000:
        relay_announced = 1
        msg_relay.display()
