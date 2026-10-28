/**
 * Scenario 19: Spatial Unit Finder (2D Range Query) — Modern eudkit (v2).
 *
 * Ideal Intent:
 * Find all units within a bounding box `[target - radius, target + radius]` in 2D space:
 * 1. Walk StarCraft's sorted Unit Finder X array (`0x66FF78`: 3400 entries of `{coord_x, unit_ptr}`).
 * 2. Early exit (`break`) as soon as `coord_x > hi_x` (exploiting sorted order).
 * 3. For entries within `[lo_x, hi_x]`:
 *    - Resolve unit EPD via `epdread_epd`.
 *    - Inspect `posY` from CUnit offset `0x2A` (`wread_epd(unit_epd + 10, 1)`).
 *    - Test if `lo_y <= pos_y <= hi_y`.
 *    - Record first match and increment `found_count`.
 */

import {
    EPD,
    dwread_epd,
    epdread_epd,
    session,
    wread_epd,
} from "eudkit";

export const UNIT_FINDER_X = 0x66FF78;
export const FINDER_BASE_EPD = EPD(UNIT_FINDER_X);
export const FINDER_ENTRIES = 3400;
export let target_x = session.var(1024, "target_x");
export let target_y = session.var(1024, "target_y");
export let radius = session.var(128, "radius");
export let found_count = session.var(0, "found_count");
export let first_found_epd = session.var(0, "first_found_epd");
export let lo_x = session.var(0, "lo_x");
export let hi_x = session.var(0, "hi_x");
export let lo_y = session.var(0, "lo_y");
export let hi_y = session.var(0, "hi_y");
export let idx = session.var(0, "idx");
export let coord = session.var(0, "coord");
export let unit_epd = session.var(0, "unit_epd");
export let pos_y = session.var(0, "pos_y");

export function main(): void {
    found_count = 0;
    first_found_epd = 0;
    lo_x = (target_x - radius);
    hi_x = (target_x + radius);
    lo_y = (target_y - radius);
    hi_y = (target_y + radius);
    idx = 0;
    while ((idx < FINDER_ENTRIES)) {
        coord = dwread_epd((FINDER_BASE_EPD + (idx * 2)));
        if ((coord > hi_x)) {
            break;
        }
        if ((coord >= lo_x)) {
            unit_epd = epdread_epd(((FINDER_BASE_EPD + (idx * 2)) + 1));
            pos_y = wread_epd((unit_epd + 10), 1);
            if ((pos_y >= lo_y)) {
                if ((pos_y <= hi_y)) {
                    if ((found_count == 0)) {
                        first_found_epd = unit_epd;
                    }
                    found_count += 1;
                }
            }
        }
        idx += 1;
    }
}
