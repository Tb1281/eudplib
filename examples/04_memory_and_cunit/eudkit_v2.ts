/**
 * Example 04: Memory & Game Structs — Modern eudkit (TypeScript).
 *
 * Demonstrates:
 * - Typed CUnit wrapper with named engine fields (hp, shields)
 * - Struct property assignment lowering to direct SetDeathsDynamic
 * - Standalone compile via `eudkit build`
 */

import { session, CUnit, EPD } from "eudkit";

export let active_unit = session.var(EPD(0x59CCA8), "active_unit");

export function main() {
    let u = new CUnit(active_unit);

    // Modify engine fields directly with idiomatic TypeScript syntax
    u.hp = 1000;
    u.shields += 50;
}
