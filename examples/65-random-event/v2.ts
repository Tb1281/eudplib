/**
 *
 */

import {
    Add,
    CreateUnit,
    Gas,
    Ore,
    Random,
    SetResources,
    Subtract,
    every,
    session,
} from "eudkit";

export const RHYNADON = 89;
export const CENTER = 40;
export const NEUTRAL = 11;
export const PERIOD = 480;
export const EVENT_COUNT = 4;
export let seed = session.var(0x75BCD15, "seed");
export let roll = session.var(0, "roll");
export let last_event = session.var(0, "last_event");
export let windfall_events = session.var(0, "windfall_events");
export let gas_events = session.var(0, "gas_events");
export let tax_events = session.var(0, "tax_events");
export let critter_events = session.var(0, "critter_events");
export let quiet_events = session.var(0, "quiet_events");
export let total_windfall_ore = session.var(0, "total_windfall_ore");
export const msg_windfall = session.map_string("Random event: mineral windfall!");
export const msg_gas = session.map_string("Random event: gas rush!");
export const msg_tax = session.map_string("Random event: tax day.");
export const msg_critters = session.map_string("Random event: critters wander in.");

export function main(): void {
    every(PERIOD, () => {
        seed = ((seed * 0x41C64E6D) + 12345);
        roll = ((seed >> 16) % 100);
        if ((roll < 15)) {
            for (let p = 0; p < EVENT_COUNT; p++) {
                SetResources(p, Add, 200, Ore);
            }
            total_windfall_ore += (200 * EVENT_COUNT);
            windfall_events += 1;
            last_event = 1;
            msg_windfall.display();
        } else if ((roll < 30)) {
            for (let p = 0; p < EVENT_COUNT; p++) {
                SetResources(p, Add, 100, Gas);
            }
            gas_events += 1;
            last_event = 2;
            msg_gas.display();
        } else if ((roll < 45)) {
            for (let p = 0; p < EVENT_COUNT; p++) {
                SetResources(p, Subtract, 50, Gas);
            }
            tax_events += 1;
            last_event = 3;
            msg_tax.display();
        } else if ((roll < 60)) {
            CreateUnit(4, RHYNADON, CENTER, NEUTRAL);
            critter_events += 1;
            last_event = 4;
            msg_critters.display();
        } else {
            quiet_events += 1;
            last_event = 5;
        }
    });
}
