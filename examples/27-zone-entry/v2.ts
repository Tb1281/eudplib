/**
 * Scenario 27: Zone Entry and Dwell Detection — Modern eudkit (v2).
 *
 * Ideal Intent:
 * Watch Player 1's Marines entering zone at location 2:
 * 1. Each entry (transition from outside to inside) awards a 25-ore bonus
 *    (first 10 entries only), pings the zone on the minimap, and increments entries.
 * 2. Each exit (transition from inside to outside) increments exits and clears inside flag.
 * 3. Every loop spent inside accumulates into `loops_inside`.
 * Modern v2 uses native Python control flow, eliminating manual EUDIf/EUDEndIf boilerplates.
 */

import {
    Add,
    AtLeast,
    AtMost,
    Bring,
    MinimapPing,
    Ore,
    Player1,
    SetResources,
    session,
} from "eudkit";

export const LOC_ZONE = 2;
export const ENTRY_BONUS = 25;
export const PAID_ENTRIES = 10;
export let inside = session.var(0, "inside");
export let entries = session.var(0, "entries");
export let exits = session.var(0, "exits");
export let paid_entries = session.var(0, "paid_entries");
export let bonus_ore_total = session.var(0, "bonus_ore_total");
export let loops_inside = session.var(0, "loops_inside");

export function main(): void {
    if (Bring(Player1, AtLeast, 1, "Terran Marine", LOC_ZONE)) {
        if ((inside == 0)) {
            entries += 1;
            if ((paid_entries < PAID_ENTRIES)) {
                paid_entries += 1;
                bonus_ore_total += ENTRY_BONUS;
                SetResources(Player1, Add, ENTRY_BONUS, Ore);
            }
            MinimapPing(LOC_ZONE);
            inside = 1;
        }
    }
    if (Bring(Player1, AtMost, 0, "Terran Marine", LOC_ZONE)) {
        if ((inside == 1)) {
            exits += 1;
            inside = 0;
        }
    }
    if ((inside == 1)) {
        loops_inside += 1;
    }
}
