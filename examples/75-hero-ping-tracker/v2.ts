/**
 *
 */

import {
    MinimapPing,
    MoveLocation,
    P1,
    dwread,
    dwread_epd,
    every,
    session,
    wread_epd,
} from "eudkit";

export const P1 = 0;
export const HERO = 20;
export const ANYWHERE = 1;
export const TRACK_LOC = 70;
export const PING_PERIOD = 48;
export const FIRST_PLAYER_UNIT = (0x6283F8 + (4 * P1));
export const DEATHS_BASE = 0x58A364;
export let coord_buf = session.string(48);
export let hero_found = session.var(0, "hero_found");
export let hero_announced_lost = session.var(0, "hero_announced_lost");
export let hero_x = session.var(0, "hero_x");
export let hero_y = session.var(0, "hero_y");
export let last_x = session.var(0, "last_x");
export let last_y = session.var(0, "last_y");
export let pings = session.var(0, "pings");
export let scans = session.var(0, "scans");
export let found_total = session.var(0, "found_total");
export let uptr = session.var(0, "uptr");
export let uepd = session.var(0, "uepd");
export const msg_lost = session.map_string("The hero is missing - no signal.");

export function main(): void {
    every(PING_PERIOD, () => {
        scans += 1;
        hero_found = 0;
        uptr = dwread(FIRST_PLAYER_UNIT);
        while ((uptr != 0)) {
            uepd = (((uptr - DEATHS_BASE) / 4) | 0);
            let _tmp1 = wread_epd((uepd + 25), 0);
            if ((_tmp1 == HERO)) {
                hero_x = wread_epd((uepd + 10), 0);
                hero_y = wread_epd((uepd + 10), 1);
                hero_found = 1;
            }
            uptr = dwread_epd((uepd + 27));
        }
        if ((hero_found == 1)) {
            last_x = hero_x;
            last_y = hero_y;
            hero_announced_lost = 0;
            MoveLocation(TRACK_LOC, HERO, P1, ANYWHERE);
            MinimapPing(TRACK_LOC);
            coord_buf.printf("HERO AT ", hero_x, ", ", hero_y);
            coord_buf.display();
            pings += 1;
            found_total += 1;
        } else if ((hero_announced_lost == 0)) {
            hero_announced_lost = 1;
            msg_lost.display();
        }
    });
}
