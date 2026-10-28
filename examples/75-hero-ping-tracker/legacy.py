# 75 - hero tracking: periodic minimap ping + coordinates (legacy)
#
# Intent: Player 1's hero (a Jim Raynor Marine, unit id 20) is
# tracked for the team. Every 48 trigger loops the tracker scans
# Player 1's unit list for the hero, reads his position from the
# CUnit posX/posY word fields, centers the tracking location on
# him (MoveLocation), pings the minimap there, and prints the
# coordinates ("HERO AT 2048, 1536") so allies can glance at the
# numbers. If the hero is dead (or not yet spawned), the tracker
# reports the loss once per disappearance instead of pinging a
# stale position, and remembers the last known position.
#
# Counting form (pilot convention): run() builds the scenario's
# triggers once; benchmarks/pilot/count_legacy.py calls it inside
# a fresh PushTriggerScope and reports the GetTriggerCounter()
# delta. DBString text needs string ids from the loaded map, so
# LoadMap happens at module level (outside the delta), exactly
# like the save-code scenario (G-68). The loop's read call sites
# ride the import-time readtable engine (session-amortized,
# outside the delta) -- the pilot RESULTS asymmetry.
from eudkit import *
from eudkit.string.dbstr import DBString

LoadMap("tests/outputmap/basemap/basemap.scx")

coord_buf = DBString(48)        # the reusable coordinate line

P1 = 0
HERO = 20                       # Jim Raynor (Marine)
ANYWHERE = 1                    # location id used as search area
TRACK_LOC = 70
PING_PERIOD = 48

# CUnit dword offsets: unitID +0x64 (word 0 of dword 25),
# pos +0x28 (posX word 0, posY word 1 of dword 10).
TYPE_EPD_OFF = 25
POS_EPD_OFF = 10

tick = EUDVariable()
hero_found = EUDVariable()      # hero seen in the latest scan
hero_announced_lost = EUDVariable()
hero_x = EUDVariable()
hero_y = EUDVariable()
last_x = EUDVariable()
last_y = EUDVariable()
pings = EUDVariable()
scans = EUDVariable()
found_total = EUDVariable()     # scans that found the hero


def scan_and_report():
    global hero_found, hero_x, hero_y, last_x, last_y
    global pings, scans, found_total, hero_announced_lost
    scans += 1
    hero_found << 0
    for ptr, epd in EUDLoopPlayerUnit(EncodePlayer(P1)):
        if EUDIf()(f_wread_epd(epd + TYPE_EPD_OFF, 0) == HERO):
            hero_x << f_wread_epd(epd + POS_EPD_OFF, 0)
            hero_y << f_wread_epd(epd + POS_EPD_OFF, 1)
            hero_found << 1
        EUDEndIf()

    if EUDIf()(hero_found == 1):
        last_x << hero_x
        last_y << hero_y
        hero_announced_lost << 0
        DoActions(MoveLocation(TRACK_LOC, HERO,
                               EncodePlayer(P1), ANYWHERE))
        DoActions(MinimapPing(TRACK_LOC))
        f_dbstr_print(coord_buf, "HERO AT ", hero_x, ", ", hero_y)
        coord_buf.Display()
        pings += 1
        found_total += 1
    EUDEndIf()
    if EUDIf()([hero_found == 0, hero_announced_lost == 0]):
        hero_announced_lost << 1
        DoActions(DisplayText(Db("The hero is missing - no signal.")))
    EUDEndIf()


def run():
    global tick
    tick += 1
    if EUDIf()(tick >= PING_PERIOD):
        tick << 0
        scan_and_report()
    EUDEndIf()
