/**
 *
 */

import {
    Array,
    EPD,
    dwread_epd,
    dwwrite_epd,
    session,
    wread_epd,
} from "eudkit";

export const CUNIT_BASE = 0x59CCA8;
export const CUNIT_EPD_BASE = EPD(CUNIT_BASE);
export const CUNIT_EPD_STRIDE = 84;
export const CUNIT_SLOTS = 1700;
export const MARINE = 0;
export const HERO_HP = 500;
export const EMPTY = 0;
export const LIVE = 1;
export const DEAD = 2;
export const shadow_state = new Array(CUNIT_SLOTS);
export const shadow_ident = new Array(CUNIT_SLOTS);
export let slot = session.var(0, "slot");
export let heroes_died = session.var(0, "heroes_died");
export let heroes_replaced = session.var(0, "heroes_replaced");
export let heroes_alive = session.var(0, "heroes_alive");
export let alive_scan = session.var(0, "alive_scan");

export function main(): void {
    while ((slot < CUNIT_SLOTS)) {
        let unit_epd = (CUNIT_EPD_BASE + (slot * CUNIT_EPD_STRIDE));
        let sprite = dwread_epd((unit_epd + 3));
        if ((sprite == 0)) {
            let _tmp1 = dwread_epd(EPD(0x6FE000 + 0) + slot);
            if ((_tmp1 == LIVE)) {
                heroes_died += 1;
                dwwrite_epd(EPD(0x6FE000 + 0) + slot, DEAD);
            }
        } else {
            let unit_type = wread_epd((unit_epd + 25), 0);
            let hp = dwread_epd((unit_epd + 2));
            let _tmp2 = dwread_epd(EPD(0x6FE000 + 0) + slot);
            if ((_tmp2 == DEAD)) {
                if ((unit_type == MARINE)) {
                    if ((hp >= HERO_HP)) {
                        heroes_replaced += 1;
                        dwwrite_epd(EPD(0x6FE000 + 0) + slot, LIVE);
                        dwwrite_epd(EPD(0x6FE000 + 1024) + slot, sprite);
                    }
                }
            }
            let _tmp3 = dwread_epd(EPD(0x6FE000 + 0) + slot);
            if ((_tmp3 == LIVE)) {
                let _tmp4 = dwread_epd(EPD(0x6FE000 + 1024) + slot);
                if ((_tmp4 != sprite)) {
                    heroes_died += 1;
                    heroes_replaced += 1;
                    dwwrite_epd(EPD(0x6FE000 + 1024) + slot, sprite);
                }
            }
            let _tmp5 = dwread_epd(EPD(0x6FE000 + 0) + slot);
            if ((_tmp5 == EMPTY)) {
                if ((unit_type == MARINE)) {
                    if ((hp >= HERO_HP)) {
                        dwwrite_epd(EPD(0x6FE000 + 0) + slot, LIVE);
                        dwwrite_epd(EPD(0x6FE000 + 1024) + slot, sprite);
                    }
                }
            }
        }
        slot += 1;
    }
    slot = 0;
    heroes_alive = 0;
    while ((alive_scan < CUNIT_SLOTS)) {
        let _tmp6 = dwread_epd(EPD(0x6FE000 + 0) + alive_scan);
        if ((_tmp6 == LIVE)) {
            heroes_alive += 1;
        }
        alive_scan += 1;
    }
    alive_scan = 0;
}
