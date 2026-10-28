// @ts-check
import { KillUnit, Player1 } from "eudkit";

/** @type {import("eudkit").Action} */
const killMarine = KillUnit("Terran Marine", Player1);

/** @type {import("eudkit").Action} */
const killZergling = KillUnit("Zerg Zergling", 1);

/**
 * @param {string} unit
 * @param {number} player
 * @returns {import("eudkit").Action}
 */
function kill(unit, player) {
  return KillUnit(unit, player);
}

// @ts-expect-error the player argument is a number, not a unit name.
KillUnit("Terran Marine", "Player1");

export { kill, killMarine, killZergling };
