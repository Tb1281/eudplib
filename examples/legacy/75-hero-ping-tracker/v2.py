# 75 - hero tracking: periodic minimap ping + coordinates (modern eudkit)
#
# Expresses ideal intent:
# 1. Consolidated `StringBuffer` API for single dynamic string architecture (ADR 0004).
# 2. Periodic unit scan via `with session.every(48):` clock rule.
# 3. Dynamic minimap ping centering via stock `MoveLocation` and coordinate reporting.
import eudkit


P1 = 0
HERO = 20
ANYWHERE = 1
TRACK_LOC = 70
PING_PERIOD = 48

FIRST_PLAYER_UNIT = 0x6283F8 + 4 * P1
DEATHS_BASE = 0x58A364

from eudkit import session

coord_buf = session.string(48)

hero_found = session.var(0)
hero_announced_lost = session.var(0)
hero_x = session.var(0)
hero_y = session.var(0)
last_x = session.var(0)
last_y = session.var(0)
pings = session.var(0)
scans = session.var(0)
found_total = session.var(0)

uptr = session.var(0)
uepd = session.var(0)

msg_lost = session.map_string("The hero is missing - no signal.")

@session.fn(main_loop=True)
def game_loop() -> None:
    global hero_found, hero_announced_lost, hero_x, hero_y, last_x, last_y
    global pings, scans, found_total, uptr, uepd

    with session.every(PING_PERIOD):
        scans += 1
        hero_found = 0

        uptr = eudkit.dwread(FIRST_PLAYER_UNIT)
        while uptr != 0:
            uepd = (uptr - DEATHS_BASE) // 4
            if eudkit.wread_epd(uepd + 25, 0) == HERO:
                hero_x = eudkit.wread_epd(uepd + 10, 0)
                hero_y = eudkit.wread_epd(uepd + 10, 1)
                hero_found = 1
            uptr = eudkit.dwread_epd(uepd + 27)

        if hero_found == 1:
            last_x = hero_x
            last_y = hero_y
            hero_announced_lost = 0
            eudkit.MoveLocation(TRACK_LOC, HERO, P1, ANYWHERE)
            eudkit.MinimapPing(TRACK_LOC)
            coord_buf.printf("HERO AT ", hero_x, ", ", hero_y)
            coord_buf.display()
            pings += 1
            found_total += 1
        elif hero_announced_lost == 0:
            hero_announced_lost = 1
            msg_lost.display()
