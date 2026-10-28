# 66 - beacon capture: ownership transfer + player color (legacy)
#
# Intent: a neutral outpost (4 SCVs and a Siege Tank at location
# 20, owned by Neutral) changes hands once: the first of the four
# players to bring a Marine to the beacon (location 21) captures
# it. The capture gives ALL outpost units to the capturer (the
# classic GiveUnits transfer), repaints the capturer's player
# color to gold through the player-color table in game memory
# (0x581D76 + player, one byte per player -- the standard EUD
# color trick), pays a 50-ore salvage reward, and announces it.
# Later arrivals do nothing: the beacon is spent.
#
# Counting form (pilot convention): run() builds the scenario's
# triggers once; benchmarks/pilot/count_legacy.py calls it inside
# a fresh PushTriggerScope and reports the GetTriggerCounter()
# delta. Units, locations and players are numeric ids (no
# basemap); text is a Db string, as in the pilot scenarios.
from eudkit import *

MARINE = 0
SCV = 7
SIEGE_TANK = 5
OUTPOST = 20
BEACON = 21
NEUTRAL = 11
COLOR_ADDR = 0x581D76    # player color table, 1 byte per player
GOLD = 0x9C              # palette index used as the capture color
SALVAGE_ORE = 50

captured = EUDVariable()
capturer = EUDVariable()          # capturing player's index
capture_loop = EUDVariable()      # loop counter value at capture
loop_count = EUDVariable()        # loops since game start
relay_announced = EUDVariable()   # 1 once the late text was shown


def attempt_capture(p):
    player = EncodePlayer(p)
    if EUDIf()([captured == 0,
                Bring(player, AtLeast, 1, MARINE, BEACON)]):
        captured << 1
        capturer << p
        capture_loop << loop_count
        # Transfer the whole outpost, unit type by unit type.
        DoActions(GiveUnits(All, SCV, EncodePlayer(NEUTRAL),
                            OUTPOST, player))
        DoActions(GiveUnits(All, SIEGE_TANK, EncodePlayer(NEUTRAL),
                            OUTPOST, player))
        # Repaint the capturer: one byte in the color table.
        f_bwrite(COLOR_ADDR + p, GOLD)
        # Salvage reward and the announcement.
        DoActions(SetResources(player, Add, SALVAGE_ORE, Ore))
        DoActions(DisplayText(
            Db("The outpost has been captured! Its units change sides.")))
    EUDEndIf()


def run():
    global loop_count
    loop_count += 1
    for p in range(4):
        attempt_capture(p)

    # Late-game flavor: long after the capture, remind everyone
    # who holds the outpost (once only).
    if EUDIf()([captured == 1, relay_announced == 0,
                loop_count >= 2000]):
        relay_announced << 1
        DoActions(DisplayText(
            Db("The gold outpost still flies its capturer's colors.")))
    EUDEndIf()
