# 61 - heal aura (modern eudkit)
#
# Expresses ideal intent:
# 1. Periodic pulse using native `with every(20):` clock rule.
# 2. Player unit list traversal (0x6283F8) with direct EPD offset addressing.
# 3. Exact HP deficit calculation and writeback without global scratch variables.
import eudkit


P1 = 0
MARINE = 0

AURA_X = 1024
AURA_Y = 1024
AURA_RANGE = 256
X_MIN = AURA_X - AURA_RANGE
X_MAX = AURA_X + AURA_RANGE
Y_MIN = AURA_Y - AURA_RANGE
Y_MAX = AURA_Y + AURA_RANGE
MARINE_MAX_HP = 10240
HEAL_AMOUNT = 512
AURA_PERIOD = 20

FIRST_PLAYER_UNIT = 0x6283F8 + 4 * P1
DEATHS_BASE = 0x58A364

from eudkit import session

scanned = session.var(0)
total_scans = session.var(0)
healed_last = session.var(0)
heals_applied = session.var(0)
hp_restored = session.var(0)

uptr = session.var(0)
uepd = session.var(0)

@session.fn(main_loop=True)
def game_loop() -> None:
    global scanned, total_scans, healed_last, heals_applied, hp_restored
    global uptr, uepd

    with session.every(AURA_PERIOD):
        scanned = 0
        healed_last = 0
        total_scans += 1

        uptr = eudkit.dwread(FIRST_PLAYER_UNIT)
        while uptr != 0:
            uepd = (uptr - DEATHS_BASE) // 4
            scanned += 1
            unit_type = eudkit.wread_epd(uepd + 25, 0)
            if unit_type == MARINE:
                x = eudkit.wread_epd(uepd + 10, 0)
                if x >= X_MIN and x <= X_MAX:
                    y = eudkit.wread_epd(uepd + 10, 1)
                    if y >= Y_MIN and y <= Y_MAX:
                        hp = eudkit.dwread_epd(uepd + 2)
                        if hp < MARINE_MAX_HP:
                            if hp <= MARINE_MAX_HP - HEAL_AMOUNT:
                                hp += HEAL_AMOUNT
                                hp_restored += HEAL_AMOUNT
                                eudkit.dwwrite_epd(uepd + 2, hp)
                            else:
                                deficit = MARINE_MAX_HP - hp
                                hp_restored += deficit
                                eudkit.dwwrite_epd(uepd + 2, MARINE_MAX_HP)
                            healed_last += 1
                            heals_applied += 1
            uptr = eudkit.dwread_epd(uepd + 27)
