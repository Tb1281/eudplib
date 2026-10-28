/**
 *
 */

import {
    Accumulate,
    Add,
    AtLeast,
    Bring,
    CreateUnit,
    MoveUnit,
    Ore,
    P1,
    Random,
    SetResources,
    Subtract,
    session,
} from "eudkit";

export const P1 = 0;
export const CIVILIAN = 15;
export const MARINE = 0;
export const BATTLECRUISER = 12;
export const SHOP = 60;
export const SHOP_EXIT = 61;
export const BASE = 30;
export const PRICE = 50;
export const COOLDOWN_LOOPS = 120;
export let cooldown = session.var(0, "cooldown");
export let seed = session.var(0x3ADE68B1, "seed");
export let roll = session.var(0, "roll");
export let boxes_opened = session.var(0, "boxes_opened");
export let consolation_count = session.var(0, "consolation_count");
export let mineral_count = session.var(0, "mineral_count");
export let army_count = session.var(0, "army_count");
export let jackpot_count = session.var(0, "jackpot_count");
export let ore_spent = session.var(0, "ore_spent");
export let ore_won_back = session.var(0, "ore_won_back");
export const msg_consolation = session.map_string("Random box: 25 ore consolation.");
export const msg_mineral = session.map_string("Random box: 100 ore mineral pack!");
export const msg_army = session.map_string("Random box: 2 Marines join you!");
export const msg_jackpot = session.map_string("Random box: JACKPOT - a Battlecruiser!");

export function main(): void {
    if ((cooldown >= 1)) {
        cooldown -= 1;
    }
    if (((cooldown == 0) && Bring(P1, AtLeast, 1, CIVILIAN, SHOP) && Accumulate(P1, AtLeast, PRICE, Ore))) {
        SetResources(P1, Subtract, PRICE, Ore);
        ore_spent += PRICE;
        MoveUnit(1, CIVILIAN, P1, SHOP, SHOP_EXIT);
        cooldown = COOLDOWN_LOOPS;
        boxes_opened += 1;
        seed = ((seed * 0x41C64E6D) + 12345);
        roll = ((seed >> 16) % 100);
        if ((roll < 50)) {
            SetResources(P1, Add, 25, Ore);
            ore_won_back += 25;
            consolation_count += 1;
            msg_consolation.display();
        } else if ((roll < 80)) {
            SetResources(P1, Add, 100, Ore);
            ore_won_back += 100;
            mineral_count += 1;
            msg_mineral.display();
        } else if ((roll < 95)) {
            CreateUnit(2, MARINE, BASE, P1);
            army_count += 1;
            msg_army.display();
        } else {
            CreateUnit(1, BATTLECRUISER, BASE, P1);
            jackpot_count += 1;
            msg_jackpot.display();
        }
    }
}
