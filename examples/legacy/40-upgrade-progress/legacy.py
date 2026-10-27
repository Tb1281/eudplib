# 40 - upgrade progress detection (legacy eudplib)
#
# Intent: watch the Terran Infantry Armor upgrade LEVEL of players 1-2.
# Whenever a player's level rises, announce the finished level and pay
# score (50 x new level); reaching Lv3 also pays +500 ore. A second
# watch tracks P1's Infantry Weapons (40 x level, no ore). A prev-level
# variable per watch turns the level (a state) into rise events.
#
# Level layout: per-player upgrade level bytes. The address follows the
# BWAPI memory map (upgrade level array based at 0x58D088, laid out
# [player][upgrade], 61 upgrades per player); both sides of this pair
# read the identical byte, and the note flags it for an in-game check.
#
# Counting form (pilot convention): run() builds the scenario's
# triggers once; benchmarks/pilot/count_legacy.py calls it inside a
# fresh PushTriggerScope and reports the GetTriggerCounter() delta.
# In production this body is the beforeTriggerExec hook. Text is a Db
# string (no basemap in the harness, like the pilot scenarios). Every
# f_bread call site rides the import-time readtable engine (80T, built
# once per session and outside the delta) -- the amortization
# asymmetry the pilot RESULTS calls out.
from eudkit import *

UPG_LEVEL_BASE = 0x58D088
UPGRADES_PER_PLAYER = 61
INFANTRY_ARMOR = 0
INFANTRY_WEAPONS = 7

prev_level = [EUDVariable() for _ in range(2)]
prev_weapons = EUDVariable()


@EUDFunc
def announce_level(player, level):
    if EUDIf()(level == 1):
        DoActions(DisplayText(Db("Infantry Armor Lv1 complete!")))
    EUDEndIf()
    if EUDIf()(level == 2):
        DoActions(DisplayText(Db("Infantry Armor Lv2 complete!")))
    EUDEndIf()
    if EUDIf()(level == 3):
        DoActions([
            DisplayText(Db("Infantry Armor Lv3 complete!")),
            SetResources(EncodePlayer(player), Add, 500, Ore),
        ])
    EUDEndIf()
    DoActions(SetScore(EncodePlayer(player), Add, 50 * level, Custom))


@EUDFunc
def announce_weapons(player, level):
    if EUDIf()(level == 1):
        DoActions(DisplayText(Db("Infantry Weapons Lv1 complete!")))
    EUDEndIf()
    if EUDIf()(level == 2):
        DoActions(DisplayText(Db("Infantry Weapons Lv2 complete!")))
    EUDEndIf()
    if EUDIf()(level == 3):
        DoActions(DisplayText(Db("Infantry Weapons Lv3 complete!")))
    EUDEndIf()
    DoActions(SetScore(EncodePlayer(player), Add, 40 * level, Custom))


def run():
    for player in range(2):
        addr = UPG_LEVEL_BASE + player * UPGRADES_PER_PLAYER + INFANTRY_ARMOR
        level = f_bread(addr)
        if EUDIf()(level > prev_level[player]):
            announce_level(player, level)
            prev_level[player] << level
        EUDEndIf()

    weapons = f_bread(UPG_LEVEL_BASE + 0 * UPGRADES_PER_PLAYER + INFANTRY_WEAPONS)
    if EUDIf()(weapons > prev_weapons):
        announce_weapons(0, weapons)
        prev_weapons << weapons
    EUDEndIf()
