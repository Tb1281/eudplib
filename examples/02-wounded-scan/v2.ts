/**
 * Scenario 02: Wounded-Unit Scan — Modern eudkit (v2).
 *
 * Ideal Intent:
 * Walk the active unit linked list every trigger cycle and census wounded units.
 * For each unit:
 * - Check unit HP (offset +0x08, EPD + 2 dwords in fixed-point 1/256 HP)
 * - Filter out mid-removal units (HP == 0)
 * - Count units below wounded threshold (< 50% HP = 5120)
 * - Count units below critical threshold (< 25% HP = 2560)
 * - Latch report variables and toggle an alarm state if any critical unit is present.
 */

import {
    EPD,
    dwread_epd,
    epdread_epd,
    session,
} from "eudkit";

export const WOUNDED_MAX = 5120;
export const CRITICAL_MAX = 2560;
export let scanned = session.var(0, "scanned");
export let wounded = session.var(0, "wounded");
export let critical = session.var(0, "critical");
export let report_scanned = session.var(0, "report_scanned");
export let report_wounded = session.var(0, "report_wounded");
export let report_critical = session.var(0, "report_critical");
export let alarm = session.var(0, "alarm");
export let cur_epd = session.var(0, "cur_epd");
export let cur_hp = session.var(0, "cur_hp");

export function main(): void {
    scanned = 0;
    wounded = 0;
    critical = 0;
    cur_epd = epdread_epd(EPD(0x628430));
    while ((cur_epd != EPD(0))) {
        cur_hp = dwread_epd((cur_epd + 2));
        if ((cur_hp > 0)) {
            scanned += 1;
            if ((cur_hp < WOUNDED_MAX)) {
                wounded += 1;
                if ((cur_hp < CRITICAL_MAX)) {
                    critical += 1;
                }
            }
        }
        cur_epd = epdread_epd((cur_epd + 1));
    }
    report_scanned = scanned;
    report_wounded = wounded;
    report_critical = critical;
    if ((critical >= 1)) {
        alarm = 1;
    } else {
        alarm = 0;
    }
}
