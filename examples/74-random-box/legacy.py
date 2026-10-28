# 74 - random box shop (legacy eudplib)
#
# Intent: a gamble shop. Player 1 parks a Civilian at the shop
# location with at least 50 ore saved up (the Accumulate
# condition): the 50 ore is taken, the civilian is teleported
# back out of the shop (so one visit = one box), a cooldown of
# 120 loops starts (enough time to walk back), and the box rolls
# on the game RNG with fixed weights:
#   roll < 50     consolation: 25 ore back (half the price)
#   50..79        mineral pack: 100 ore
#   80..94        army pack: 2 Marines at the player base
#   95..99        jackpot: 1 Battlecruiser at the player base
# Bookkeeping counts boxes, each tier, and the jackpots.
#
# Counting form (pilot convention): run() builds the scenario's
# triggers once; benchmarks/pilot/count_legacy.py calls it inside
# a fresh PushTriggerScope and reports the GetTriggerCounter()
# delta. Units, locations and players are numeric ids (no
# basemap); text is a Db string, as in the pilot scenarios. The
# f_dwrand call site rides legacy's EUDFunc machinery (body
# created at its first call in the session; see note.md).
from eudkit import *

P1 = 0
CIVILIAN = 15
MARINE = 0
BATTLECRUISER = 12
SHOP = 60
SHOP_EXIT = 61
BASE = 30
PRICE = 50
COOLDOWN_LOOPS = 120

cooldown = EUDVariable()
roll = EUDVariable()               # scratch: the current draw
boxes_opened = EUDVariable()
consolation_count = EUDVariable()
mineral_count = EUDVariable()
army_count = EUDVariable()
jackpot_count = EUDVariable()
ore_spent = EUDVariable()
ore_won_back = EUDVariable()       # ore paid out by the ore tiers


def open_box():
    global roll, boxes_opened, consolation_count, mineral_count
    global army_count, jackpot_count, ore_spent, ore_won_back
    global cooldown
    p1 = EncodePlayer(P1)
    DoActions(SetResources(p1, Subtract, PRICE, Ore))
    ore_spent += PRICE
    # One visit, one box: send the buyer back out.
    DoActions(MoveUnit(1, CIVILIAN, p1, SHOP, SHOP_EXIT))
    cooldown << COOLDOWN_LOOPS
    boxes_opened += 1

    roll << f_dwrand() % 100
    if EUDIf()(roll < 50):
        DoActions(SetResources(p1, Add, 25, Ore))
        ore_won_back += 25
        consolation_count += 1
        DoActions(DisplayText(Db("Random box: 25 ore consolation.")))
    if EUDElseIf()(roll < 80):
        DoActions(SetResources(p1, Add, 100, Ore))
        ore_won_back += 100
        mineral_count += 1
        DoActions(DisplayText(Db("Random box: 100 ore mineral pack!")))
    if EUDElseIf()(roll < 95):
        DoActions(CreateUnit(2, MARINE, BASE, p1))
        army_count += 1
        DoActions(DisplayText(Db("Random box: 2 Marines join you!")))
    if EUDElse()():
        DoActions(CreateUnit(1, BATTLECRUISER, BASE, p1))
        jackpot_count += 1
        DoActions(DisplayText(Db("Random box: JACKPOT - a Battlecruiser!")))
    EUDEndIf()


def run():
    global cooldown
    if EUDIf()(cooldown >= 1):
        cooldown -= 1
    EUDEndIf()

    # The purchase gate: buyer present, price affordable, ready.
    p1 = EncodePlayer(P1)
    if EUDIf()([cooldown == 0,
                Bring(p1, AtLeast, 1, CIVILIAN, SHOP),
                Accumulate(p1, AtLeast, PRICE, Ore)]):
        open_box()
    EUDEndIf()
