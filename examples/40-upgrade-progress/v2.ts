/**
 *
 */

import {
    Add,
    Custom,
    DisplayText,
    Ore,
    SetResources,
    SetScore,
    bread,
    session,
} from "eudkit";

export const UPG_LEVEL_BASE = 0x58D088;
export const UPGRADES_PER_PLAYER = 61;
export const INFANTRY_ARMOR = 0;
export const INFANTRY_WEAPONS = 7;
export const prev_armor = [session.var(0, "prev_armor_0"), session.var(0, "prev_armor_1")];
export let prev_weapons = session.var(0, "prev_weapons");

export function main(): void {
    for (let player = 0; player < 2; player++) {
        let level = bread(((UPG_LEVEL_BASE + (player * UPGRADES_PER_PLAYER)) + INFANTRY_ARMOR));
        if ((level > prev_armor[player])) {
            if ((level == 1)) {
                DisplayText("Infantry Armor Lv1 complete!");
                SetScore(player, Add, 50, Custom);
            } else if ((level == 2)) {
                DisplayText("Infantry Armor Lv2 complete!");
                SetScore(player, Add, 100, Custom);
            } else if ((level == 3)) {
                DisplayText("Infantry Armor Lv3 complete!");
                SetResources(player, Add, 500, Ore);
                SetScore(player, Add, 150, Custom);
            }
            prev_armor[player] = level;
        }
    }
    let weapons = bread(((UPG_LEVEL_BASE + (0 * UPGRADES_PER_PLAYER)) + INFANTRY_WEAPONS));
    if ((weapons > prev_weapons)) {
        if ((weapons == 1)) {
            DisplayText("Infantry Weapons Lv1 complete!");
            SetScore(0, Add, 40, Custom);
        } else if ((weapons == 2)) {
            DisplayText("Infantry Weapons Lv2 complete!");
            SetScore(0, Add, 80, Custom);
        } else if ((weapons == 3)) {
            DisplayText("Infantry Weapons Lv3 complete!");
            SetScore(0, Add, 120, Custom);
        }
        prev_weapons = weapons;
    }
}
