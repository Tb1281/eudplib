/**
 *
 */

import {
    P1,
    dwread,
    dwread_epd,
    dwwrite_epd,
    every,
    session,
    wread_epd,
} from "eudkit";

export const P1 = 0;
export const MARINE = 0;
export const AURA_X = 1024;
export const AURA_Y = 1024;
export const AURA_RANGE = 256;
export const X_MIN = (AURA_X - AURA_RANGE);
export const X_MAX = (AURA_X + AURA_RANGE);
export const Y_MIN = (AURA_Y - AURA_RANGE);
export const Y_MAX = (AURA_Y + AURA_RANGE);
export const MARINE_MAX_HP = 10240;
export const HEAL_AMOUNT = 512;
export const AURA_PERIOD = 20;
export const FIRST_PLAYER_UNIT = (0x6283F8 + (4 * P1));
export const DEATHS_BASE = 0x58A364;
export let scanned = session.var(0, "scanned");
export let total_scans = session.var(0, "total_scans");
export let healed_last = session.var(0, "healed_last");
export let heals_applied = session.var(0, "heals_applied");
export let hp_restored = session.var(0, "hp_restored");
export let uptr = session.var(0, "uptr");
export let uepd = session.var(0, "uepd");

export function main(): void {
    every(AURA_PERIOD, () => {
        scanned = 0;
        healed_last = 0;
        total_scans += 1;
        uptr = dwread(FIRST_PLAYER_UNIT);
        while ((uptr != 0)) {
            uepd = (((uptr - DEATHS_BASE) / 4) | 0);
            scanned += 1;
            let unit_type = wread_epd((uepd + 25), 0);
            if ((unit_type == MARINE)) {
                let x = wread_epd((uepd + 10), 0);
                if (((x >= X_MIN) && (x <= X_MAX))) {
                    let y = wread_epd((uepd + 10), 1);
                    if (((y >= Y_MIN) && (y <= Y_MAX))) {
                        let hp = dwread_epd((uepd + 2));
                        if ((hp < MARINE_MAX_HP)) {
                            if ((hp <= (MARINE_MAX_HP - HEAL_AMOUNT))) {
                                hp += HEAL_AMOUNT;
                                hp_restored += HEAL_AMOUNT;
                                dwwrite_epd((uepd + 2), hp);
                            } else {
                                let deficit = (MARINE_MAX_HP - hp);
                                hp_restored += deficit;
                                dwwrite_epd((uepd + 2), MARINE_MAX_HP);
                            }
                            healed_last += 1;
                            heals_applied += 1;
                        }
                    }
                }
            }
            uptr = dwread_epd((uepd + 27));
        }
    });
}
