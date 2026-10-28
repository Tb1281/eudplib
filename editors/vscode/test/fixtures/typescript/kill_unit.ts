import { Action, KillUnit, Player1 } from "eudkit";

const killMarine: Action = KillUnit("Terran Marine", Player1);

const killZergling: Action = KillUnit("Zerg Zergling", 1);

// @ts-expect-error the player argument is a number, not a unit name.
KillUnit("Terran Marine", "Player1");

export { killMarine, killZergling };
