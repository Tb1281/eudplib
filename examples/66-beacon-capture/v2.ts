/**
 *
 */

import {
    Add,
    All,
    AtLeast,
    Bring,
    GiveUnits,
    Ore,
    SetResources,
    bwrite,
    session,
} from "eudkit";

export const MARINE = 0;
export const SCV = 7;
export const SIEGE_TANK = 5;
export const OUTPOST = 20;
export const BEACON = 21;
export const NEUTRAL = 11;
export const COLOR_ADDR = 0x581D76;
export const GOLD = 156;
export const SALVAGE_ORE = 50;
export let captured = session.var(0, "captured");
export let capturer = session.var(0, "capturer");
export let capture_loop = session.var(0, "capture_loop");
export let loop_count = session.var(0, "loop_count");
export let relay_announced = session.var(0, "relay_announced");
export const msg_captured = session.map_string("The outpost has been captured! Its units change sides.");
export const msg_relay = session.map_string("The gold outpost still flies its capturer's colors.");

export function main(): void {
    loop_count += 1;
    for (let p = 0; p < 4; p++) {
        if (((captured == 0) && Bring(p, AtLeast, 1, MARINE, BEACON))) {
            captured = 1;
            capturer = p;
            capture_loop = loop_count;
            GiveUnits(All, SCV, NEUTRAL, OUTPOST, p);
            GiveUnits(All, SIEGE_TANK, NEUTRAL, OUTPOST, p);
            bwrite((COLOR_ADDR + p), GOLD);
            SetResources(p, Add, SALVAGE_ORE, Ore);
            msg_captured.display();
        }
    }
    if (((captured == 1) && (relay_announced == 0) && (loop_count >= 2000))) {
        relay_announced = 1;
        msg_relay.display();
    }
}
