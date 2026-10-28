/**
 *
 */

import {
    Add,
    AtLeast,
    Command,
    DisplayText,
    Gas,
    IsUserCP,
    Ore,
    SetResources,
    dwwrite,
    every,
    session,
} from "eudkit";

export const MARINE = 0;
export const MARINES_REQUIRED = 3;
export const BONUS_ORE = 25;
export const ELITE_MARINES = 10;
export const ELITE_GAS = 10;
export const ROUND_INTERVAL = 252;

export function main(): void {
    every(ROUND_INTERVAL, () => {
        for (let player = 0; player < 8; player++) {
            if (Command(player, AtLeast, MARINES_REQUIRED, MARINE)) {
                SetResources(player, Add, BONUS_ORE, Ore);
                dwwrite(0x6509B0, player);
                if (IsUserCP()) {
                    DisplayText("Marine corps bonus paid: +25 ore.");
                }
                if (Command(player, AtLeast, ELITE_MARINES, MARINE)) {
                    SetResources(player, Add, ELITE_GAS, Gas);
                    dwwrite(0x6509B0, player);
                    if (IsUserCP()) {
                        DisplayText("Elite corps bonus paid: +10 gas.");
                    }
                }
            }
        }
    });
}
