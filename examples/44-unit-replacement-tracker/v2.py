# 44 - unit replacement / transformation tracker (modern eudkit)
#
# Expresses ideal intent:
# 1. High-level CUnit slot scanner leveraging typed session Array storage.
# 2. Direct EPD offset addressing (dwread_epd / wread_epd) with zero runtime decryption overhead.
# 3. Clean local session state with deterministic storage layout.
import eudkit


CUNIT_BASE = 0x59CCA8
CUNIT_EPD_BASE = eudkit.EPD(CUNIT_BASE)
CUNIT_EPD_STRIDE = 84   # 336 bytes / 4 = 84 dwords
CUNIT_SLOTS = 1700
MARINE = 0
HERO_HP = 500

EMPTY = 0
LIVE = 1
DEAD = 2

from eudkit import session

shadow_state = eudkit.Array(CUNIT_SLOTS)
shadow_ident = eudkit.Array(CUNIT_SLOTS)

slot = session.var(0)
heroes_died = session.var(0)
heroes_replaced = session.var(0)
heroes_alive = session.var(0)
alive_scan = session.var(0)

@session.fn(main_loop=True)
def game_loop() -> None:
    global slot, heroes_died, heroes_replaced, heroes_alive, alive_scan
    while slot < CUNIT_SLOTS:
        unit_epd = CUNIT_EPD_BASE + slot * CUNIT_EPD_STRIDE
        sprite = eudkit.dwread_epd(unit_epd + 3)

        if sprite == 0:
            if shadow_state[slot] == LIVE:
                heroes_died += 1
                shadow_state[slot] = DEAD
        else:
            unit_type = eudkit.wread_epd(unit_epd + 25, 0)
            hp = eudkit.dwread_epd(unit_epd + 2)

            if shadow_state[slot] == DEAD:
                if unit_type == MARINE:
                    if hp >= HERO_HP:
                        heroes_replaced += 1
                        shadow_state[slot] = LIVE
                        shadow_ident[slot] = sprite

            if shadow_state[slot] == LIVE:
                if shadow_ident[slot] != sprite:
                    heroes_died += 1
                    heroes_replaced += 1
                    shadow_ident[slot] = sprite

            if shadow_state[slot] == EMPTY:
                if unit_type == MARINE:
                    if hp >= HERO_HP:
                        shadow_state[slot] = LIVE
                        shadow_ident[slot] = sprite

        slot += 1
    slot = 0

    heroes_alive = 0
    while alive_scan < CUNIT_SLOTS:
        if shadow_state[alive_scan] == LIVE:
            heroes_alive += 1
        alive_scan += 1
    alive_scan = 0
