/**
 * Scenario 21: Existing vs Completed Unit Counts — Modern eudkit (v2).
 *
 * Ideal Intent:
 * Evaluate unit counts for Player 1 across both existing (under construction / morphed)
 * and completed states:
 * 1. Terran Marines existing >= 1 -> flag 208 (`Command`).
 * 2. Terran Siege Tanks existing >= 3 -> flag 209 (`Command`).
 * 3. Terran Marines completed >= 1 -> flag 210 (direct query on Completed Table at `0x584DE4`).
 * 4. Terran Siege Tanks completed >= 3 -> flag 211.
 */

import {
    AtLeast,
    Command,
    Memory,
    Player1,
    SetDeaths,
    SetTo,
    session,
} from "eudkit";

export const COMPLETED_BASE = 0x584DE4;

function counts_addr(base: number, player: number, unit: number): number {
    return ((base + (48 * unit)) + (4 * player));
}

export function main(): void {
    if (Command(Player1, AtLeast, 1, "Terran Marine")) {
        SetDeaths(Player1, SetTo, 1, 208);
    }
    if (Command(Player1, AtLeast, 3, "Terran Siege Tank (Tank Mode)")) {
        SetDeaths(Player1, SetTo, 1, 209);
    }
    if (Memory((COMPLETED_BASE + 48 * 0 + 4 * 0), AtLeast, 1)) {
        SetDeaths(Player1, SetTo, 1, 210);
    }
    if (Memory((COMPLETED_BASE + 48 * 5 + 4 * 0), AtLeast, 3)) {
        SetDeaths(Player1, SetTo, 1, 211);
    }
}
