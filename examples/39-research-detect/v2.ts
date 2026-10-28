/**
 *
 */

import {
    Add,
    DisplayText,
    Gas,
    Ore,
    SetResources,
    bread,
    once,
    session,
} from "eudkit";

export const RESEARCHED_BASE = 0x58CF44;
export const STIM_PACKS = 0;
export const LOCKDOWN = 1;
export const BONUS_GAS = 100;
export const LOCKDOWN_BONUS_ORE = 50;

export function main(): void {
    for (let player = 0; player < 4; player++) {
        let stim_state = bread(((RESEARCHED_BASE + (player * 24)) + STIM_PACKS));
        if ((stim_state != 0)) {
            once(() => {
                SetResources(player, Add, BONUS_GAS, Gas);
                DisplayText("Stim Packs research complete! +100 gas.");
            });
        }
        let lockdown_state = bread(((RESEARCHED_BASE + (player * 24)) + LOCKDOWN));
        if ((lockdown_state != 0)) {
            once(() => {
                SetResources(player, Add, LOCKDOWN_BONUS_ORE, Ore);
                DisplayText("Lockdown research complete! +50 ore.");
            });
        }
    }
}
