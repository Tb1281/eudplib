/**
 * Example 06: Reactive Rules & Events — Modern eudkit (TypeScript).
 *
 * Demonstrates:
 * - Declarative reactive primitives:
 *   - once(() => ...): Executes once on game start or when condition matches
 *   - every(ticks, () => ...): Repeats cyclically every N ticks
 *   - after(ticks, () => ...): Fires once after an elapsed tick delay
 *   - when(cond, () => ...): Continuous trigger while condition holds
 *   - until(cond, () => ...): Continues executing until condition becomes true
 * - Zero manual timer variable management
 * - Zero desynchronization risk (no fragile Wait() actions)
 * - Standalone compile via `eudkit build`
 */

import {
    session,
    once,
    every,
    after,
    when,
    until,
    DisplayText,
    SetResources,
    CreateUnit,
    Player1,
    SetTo,
    Add,
    Ore
} from "eudkit";

export let kills = session.var(0, "kills");

export function main() {
    // 1. One-time initialization on game start
    once(() => {
        DisplayText("Welcome to StarCraft EUD!");
        SetResources(Player1, SetTo, 1000, Ore);
    });

    // 2. Periodic passive income: every 24 ticks (~1 second on Fastest)
    every(24, () => {
        SetResources(Player1, Add, 50, Ore);
    });

    // 3. Timed reinforcement drop: arrives after 120 ticks (~5 seconds)
    after(120, () => {
        DisplayText("Reinforcements have arrived!");
        CreateUnit(1, "Terran Marine", "Anywhere", Player1);
    });

    // 4. Continuous rule while kills >= 10
    when(kills.get() >= 10, () => {
        DisplayText("Milestone reached: 10 kills!");
    });
}
