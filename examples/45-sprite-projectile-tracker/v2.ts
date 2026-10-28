/**
 *
 */

import {
    dwread,
    dwread_epd,
    session,
    wread_epd,
} from "eudkit";

export const SPRITE_LIST_HEAD = 0x629D98;
export const DEATHS_BASE = 0x58A364;
export const ARENA_MIN = 512;
export const ARENA_MAX = 3584;
export let sprite_count = session.var(0, "sprite_count");
export let sum_x = session.var(0, "sum_x");
export let sum_y = session.var(0, "sum_y");
export let centroid_x = session.var(0, "centroid_x");
export let centroid_y = session.var(0, "centroid_y");
export let prev_count = session.var(0, "prev_count");
export let appeared_total = session.var(0, "appeared_total");
export let largest_cluster = session.var(0, "largest_cluster");
export let curr_ptr = session.var(0, "curr_ptr");
export let curr_epd = session.var(0, "curr_epd");

export function main(): void {
    sprite_count = 0;
    sum_x = 0;
    sum_y = 0;
    curr_ptr = dwread(SPRITE_LIST_HEAD);
    while ((curr_ptr != 0)) {
        curr_epd = (((curr_ptr - DEATHS_BASE) / 4) | 0);
        let x = wread_epd((curr_epd + 3), 0);
        let y = wread_epd((curr_epd + 3), 1);
        if ((x >= ARENA_MIN)) {
            if ((x <= ARENA_MAX)) {
                if ((y >= ARENA_MIN)) {
                    if ((y <= ARENA_MAX)) {
                        sprite_count += 1;
                        sum_x += x;
                        sum_y += y;
                    }
                }
            }
        }
        curr_ptr = dwread_epd((curr_epd + 1));
    }
    if ((sprite_count > 0)) {
        centroid_x = ((sum_x / sprite_count) | 0);
        centroid_y = ((sum_y / sprite_count) | 0);
    }
    if ((sprite_count > prev_count)) {
        appeared_total += (sprite_count - prev_count);
        if ((sprite_count > largest_cluster)) {
            largest_cluster = sprite_count;
        }
    }
    prev_count = sprite_count;
}
