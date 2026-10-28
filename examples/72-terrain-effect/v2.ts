/**
 *
 */

import {
    dwread,
    dwread_epd,
    dwwrite_epd,
    every,
    session,
    wread_epd,
} from "eudkit";

export const TICK_PERIOD = 12;
export const LAVA_DAMAGE = 512;
export const SPRING_HEAL = 256;
export const HEAL_CAP = 10240;
export const LAVA_X0 = 512;
export const LAVA_X1 = 1024;
export const LAVA_Y0 = 2048;
export const LAVA_Y1 = 2560;
export const SPRING_X0 = 3072;
export const SPRING_X1 = 3584;
export const SPRING_Y0 = 512;
export const SPRING_Y1 = 1024;
export const FIRST_VISIBLE_UNIT = 0x628430;
export const DEATHS_BASE = 0x58A364;
export let lava_hits = session.var(0, "lava_hits");
export let lava_kills = session.var(0, "lava_kills");
export let heals = session.var(0, "heals");
export let total_damage = session.var(0, "total_damage");
export let uptr = session.var(0, "uptr");
export let uepd = session.var(0, "uepd");
export let scratch_hp = session.var(0, "scratch_hp");

export function main(): void {
    every(TICK_PERIOD, () => {
        uptr = dwread(FIRST_VISIBLE_UNIT);
        while ((uptr != 0)) {
            uepd = (((uptr - DEATHS_BASE) / 4) | 0);
            let x = wread_epd((uepd + 10), 0);
            let y = wread_epd((uepd + 10), 1);
            if (((x >= LAVA_X0) && (x <= LAVA_X1) && (y >= LAVA_Y0) && (y <= LAVA_Y1))) {
                scratch_hp = dwread_epd((uepd + 2));
                if ((scratch_hp > LAVA_DAMAGE)) {
                    dwwrite_epd((uepd + 2), (scratch_hp - LAVA_DAMAGE));
                } else {
                    dwwrite_epd((uepd + 2), 0);
                    lava_kills += 1;
                }
                lava_hits += 1;
                total_damage += LAVA_DAMAGE;
            }
            if (((x >= SPRING_X0) && (x <= SPRING_X1) && (y >= SPRING_Y0) && (y <= SPRING_Y1))) {
                scratch_hp = dwread_epd((uepd + 2));
                if ((scratch_hp < HEAL_CAP)) {
                    dwwrite_epd((uepd + 2), (scratch_hp + SPRING_HEAL));
                    heals += 1;
                }
            }
            uptr = dwread_epd((uepd + 1));
        }
    });
}
