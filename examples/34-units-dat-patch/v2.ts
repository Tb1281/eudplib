/**
 * Scenario 34: Units.dat In-Memory Patch and Verification — Modern eudkit (v2).
 *
 * Ideal Intent:
 * At map initialization, patch Terran infantry stats in `Units.dat` table memory:
 * - Marine (0): 80 HP, 1 Armor
 * - Ghost (1): 60 HP, 1 Armor
 * - Firebat (32): 70 HP, 1 Armor
 * - Medic (34): 75 HP, 1 Armor
 * HitPoints are stored shifted by 8 bits (`HP * 256`) at `0x662350 + 4 * unit_id`.
 * Armor is stored as an 8-bit byte at `0x65FEC8 + unit_id`.
 * After writing, reads back all values via unified ReadTable to verify patches stuck:
 * - If all match: announces `"Terran infantry buffed: HP up, armor 1."`
 * - Otherwise: announces `"WARNING: infantry buff did not stick."`
 * Modern v2 uses:
 * - Declarative `with once():` lifecycle block.
 * - Direct memory I/O primitives (`dwwrite`, `bwrite`, `dwread`, `bread`).
 * - Direct string literals in `DisplayText` backed by STRx deduplication.
 */

import {
    DisplayText,
    bread,
    bwrite,
    dwread,
    dwwrite,
    once,
    session,
} from "eudkit";

export const HP_BASE = 0x662350;
export const ARMOR_BASE = 0x65FEC8;
export const MARINE = 0;
export const GHOST = 1;
export const FIREBAT = 32;
export const MEDIC = 34;
export let verify_failed = session.var(0, "verify_failed");

export function main(): void {
    once(() => {
        verify_failed = 0;
        dwwrite((HP_BASE + (4 * MARINE)), (80 * 256));
        bwrite((ARMOR_BASE + MARINE), 1);
        dwwrite((HP_BASE + (4 * GHOST)), (60 * 256));
        bwrite((ARMOR_BASE + GHOST), 1);
        dwwrite((HP_BASE + (4 * FIREBAT)), (70 * 256));
        bwrite((ARMOR_BASE + FIREBAT), 1);
        dwwrite((HP_BASE + (4 * MEDIC)), (75 * 256));
        bwrite((ARMOR_BASE + MEDIC), 1);
        let hp0 = dwread((HP_BASE + (4 * MARINE)));
        let arm0 = bread((ARMOR_BASE + MARINE));
        if ((hp0 != (80 * 256))) {
            verify_failed += 1;
        }
        if ((arm0 != 1)) {
            verify_failed += 1;
        }
        let hp1 = dwread((HP_BASE + (4 * GHOST)));
        let arm1 = bread((ARMOR_BASE + GHOST));
        if ((hp1 != (60 * 256))) {
            verify_failed += 1;
        }
        if ((arm1 != 1)) {
            verify_failed += 1;
        }
        let hp32 = dwread((HP_BASE + (4 * FIREBAT)));
        let arm32 = bread((ARMOR_BASE + FIREBAT));
        if ((hp32 != (70 * 256))) {
            verify_failed += 1;
        }
        if ((arm32 != 1)) {
            verify_failed += 1;
        }
        let hp34 = dwread((HP_BASE + (4 * MEDIC)));
        let arm34 = bread((ARMOR_BASE + MEDIC));
        if ((hp34 != (75 * 256))) {
            verify_failed += 1;
        }
        if ((arm34 != 1)) {
            verify_failed += 1;
        }
        if ((verify_failed == 0)) {
            DisplayText("Terran infantry buffed: HP up, armor 1.");
        } else {
            DisplayText("WARNING: infantry buff did not stick.");
        }
    });
}
