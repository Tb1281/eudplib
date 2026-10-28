/**
 * Scenario 35: Weapons.dat and Upgrades.dat In-Memory Patch — Modern eudkit (v2).
 *
 * Ideal Intent:
 * At map initialization, patch weapons and upgrade costs in memory:
 * 1. Gauss Rifle (weapon 0): 22 damage (word), 18 cooldown (byte).
 * 2. C-10 Rifle (weapon 2): 30 damage (word), 20 cooldown (byte).
 * 3. Terran Infantry Armor (upgrade 0):
 *    - Reads current mineral base cost; if >= 100, discounts by 25.
 *    - Pins max upgrade level to 3.
 * 4. Reads back weapon values to verify patches:
 *    - If all match: announces `"Weapons tuned; infantry armor discounted."`
 * Modern v2 uses:
 * - Declarative `with once():` lifecycle block.
 * - Direct typed memory word/byte primitives (`wwrite`, `bwrite`, `wread`, `bread`).
 * - STRx-backed literal strings in `DisplayText`.
 */

import {
    DisplayText,
    bread,
    bwrite,
    once,
    session,
    wread,
    wwrite,
} from "eudkit";

export const DMG_BASE = 0x656EB0;
export const CD_BASE = 0x656FB8;
export const MINCOST_BASE = 0x655740;
export const MAXLV_BASE = 0x655700;
export const GAUSS_RIFLE = 0;
export const C10_RIFLE = 2;
export const INFANTRY_ARMOR = 0;
export let verify_failed = session.var(0, "verify_failed");

export function main(): void {
    once(() => {
        verify_failed = 0;
        wwrite((DMG_BASE + (2 * GAUSS_RIFLE)), 22);
        bwrite((CD_BASE + GAUSS_RIFLE), 18);
        wwrite((DMG_BASE + (2 * C10_RIFLE)), 30);
        bwrite((CD_BASE + C10_RIFLE), 20);
        let cost = wread((MINCOST_BASE + (2 * INFANTRY_ARMOR)));
        if ((cost >= 100)) {
            wwrite((MINCOST_BASE + (2 * INFANTRY_ARMOR)), (cost - 25));
        }
        let maxlv = bread((MAXLV_BASE + INFANTRY_ARMOR));
        if ((maxlv != 3)) {
            bwrite((MAXLV_BASE + INFANTRY_ARMOR), 3);
        }
        let dmg0 = wread((DMG_BASE + (2 * GAUSS_RIFLE)));
        let cd0 = bread((CD_BASE + GAUSS_RIFLE));
        if ((dmg0 != 22)) {
            verify_failed += 1;
        }
        if ((cd0 != 18)) {
            verify_failed += 1;
        }
        let dmg2 = wread((DMG_BASE + (2 * C10_RIFLE)));
        let cd2 = bread((CD_BASE + C10_RIFLE));
        if ((dmg2 != 30)) {
            verify_failed += 1;
        }
        if ((cd2 != 20)) {
            verify_failed += 1;
        }
        if ((verify_failed == 0)) {
            DisplayText("Weapons tuned; infantry armor discounted.");
        }
    });
}
