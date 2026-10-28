/**
 *
 */

import {
    Accumulate,
    Add,
    AtLeast,
    AtMost,
    DisplayText,
    Gas,
    Ore,
    SetResources,
    Subtract,
    every,
    session,
} from "eudkit";

export const INTERVAL = 84;
export const LOW_MINERAL = 499;
export const BONUS = 150;
export const HIGH_GAS = 301;
export const TAX = 50;
export let paid_count = session.var(0, "paid_count");

export function main(): void {
    every(INTERVAL, () => {
        paid_count = 0;
        for (let player = 0; player < 8; player++) {
            if (Accumulate(player, AtMost, LOW_MINERAL, Ore)) {
                SetResources(player, Add, BONUS, Ore);
                paid_count += 1;
            }
            if (Accumulate(player, AtLeast, HIGH_GAS, Gas)) {
                SetResources(player, Subtract, TAX, Gas);
            }
        }
        if ((paid_count >= 1)) {
            DisplayText("Relief payments issued.");
        }
    });
}
