/**
 *
 */

import {
    AtLeast,
    AtMost,
    Attack,
    Command,
    CreateUnit,
    Move,
    Order,
    Patrol,
    every,
    session,
} from "eudkit";

export const MARINE = 0;
export const ENEMY = 7;
export const ENEMY_BASE = 50;
export const MUSTER_FIELD = 52;
export const PLAYER_BASE = 51;
export const MIDFIELD = 54;
export const RALLY = 53;
export const DECISION_PERIOD = 90;
export const ATTACK_THRESHOLD = 12;
export const RETREAT_THRESHOLD = 3;
export const REINFORCE_CAP = 20;
export const REINFORCE_BATCH = 2;
export let attack_waves = session.var(0, "attack_waves");
export let patrol_orders = session.var(0, "patrol_orders");
export let retreats = session.var(0, "retreats");
export let reinforcements = session.var(0, "reinforcements");
export let last_stance = session.var(0, "last_stance");

export function main(): void {
    every(DECISION_PERIOD, () => {
        if (Command(ENEMY, AtLeast, ATTACK_THRESHOLD, MARINE)) {
            Order(MARINE, ENEMY, MUSTER_FIELD, Attack, PLAYER_BASE);
            attack_waves += 1;
            last_stance = 1;
        } else if (Command(ENEMY, AtLeast, (RETREAT_THRESHOLD + 1), MARINE)) {
            Order(MARINE, ENEMY, MUSTER_FIELD, Patrol, MIDFIELD);
            patrol_orders += 1;
            last_stance = 2;
        } else {
            Order(MARINE, ENEMY, MUSTER_FIELD, Move, RALLY);
            retreats += 1;
            last_stance = 3;
        }
        if (Command(ENEMY, AtMost, (REINFORCE_CAP - 1), MARINE)) {
            CreateUnit(REINFORCE_BATCH, MARINE, ENEMY_BASE, ENEMY);
            reinforcements += REINFORCE_BATCH;
        }
    });
}
