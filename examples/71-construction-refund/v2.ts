/**
 *
 */

import {
    P1,
    dwread,
    dwwrite,
    once,
    session,
} from "eudkit";

export const P1 = 0;
export const BARRACKS = 111;
export const BARRACKS_COST = 150;
export const ALL_COUNTS = 0x582324;
export const DONE_COUNTS = 0x584DE4;
export const ORE_ADDR = 0x57F0F0;
export let existing = session.var(0, "existing");
export let completed = session.var(0, "completed");
export let prev_building = session.var(0, "prev_building");
export let prev_completed = session.var(0, "prev_completed");
export let started_total = session.var(0, "started_total");
export let finished_total = session.var(0, "finished_total");
export let cancelled_total = session.var(0, "cancelled_total");
export let refunded_total = session.var(0, "refunded_total");

export function main(): void {
    existing = dwread(((ALL_COUNTS + (48 * BARRACKS)) + (4 * P1)));
    completed = dwread(((DONE_COUNTS + (48 * BARRACKS)) + (4 * P1)));
    let building = (existing - completed);
    once(() => {
        prev_building = building;
        prev_completed = completed;
    });
    let d_completed = 0;
    if ((completed > prev_completed)) {
        d_completed = (completed - prev_completed);
        finished_total += d_completed;
    }
    if ((building < prev_building)) {
        let d_building = (prev_building - building);
        if ((d_building > d_completed)) {
            let cancelled_now = (d_building - d_completed);
            cancelled_total += cancelled_now;
            let refund_amount = (cancelled_now * BARRACKS_COST);
            let ore_now = dwread((ORE_ADDR + (4 * P1)));
            dwwrite((ORE_ADDR + (4 * P1)), (ore_now + refund_amount));
            refunded_total += refund_amount;
        }
    }
    if ((building > prev_building)) {
        started_total += (building - prev_building);
    }
    prev_building = building;
    prev_completed = completed;
}
