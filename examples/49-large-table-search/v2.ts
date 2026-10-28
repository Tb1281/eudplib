/**
 *
 */

import {
    Array,
    EPD,
    dwread_epd,
    dwwrite_epd,
    session,
} from "eudkit";

export const TABLE_N = 512;
export const NOT_FOUND = (-1);
export const item_key = new Array(TABLE_N);
export const item_price = new Array(TABLE_N);
export let search_key = session.var(0, "search_key");
export let search_request = session.var(0, "search_request");
export let found_index = session.var(NOT_FOUND, "found_index");
export let found_price = session.var(0, "found_price");
export let searches_done = session.var(0, "searches_done");
export let not_found_count = session.var(0, "not_found_count");
export let total_probes = session.var(0, "total_probes");
export let best_price = session.var(0x7FFFFFFF, "best_price");
export let idx = session.var(0, "idx");
export let probes = session.var(0, "probes");

export function main(): void {
    if ((search_request == 1)) {
        idx = 0;
        probes = 0;
        found_index = NOT_FOUND;
        while ((idx < TABLE_N)) {
            probes += 1;
            let _tmp1 = dwread_epd(EPD(0x6FE000 + 0) + idx);
            if ((_tmp1 == search_key)) {
                found_index = idx;
                let _tmp2 = dwread_epd(EPD(0x6FE000 + 1024) + idx);
                found_price = _tmp2;
                break;
            }
            idx += 1;
        }
        if ((found_index == NOT_FOUND)) {
            not_found_count += 1;
        } else if ((found_price < best_price)) {
            best_price = found_price;
        }
        total_probes += probes;
        searches_done += 1;
        search_request = 0;
    }
}
