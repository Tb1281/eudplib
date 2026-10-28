/**
 * Example 02: Control Flow — Modern eudkit (TypeScript).
 *
 * Demonstrates:
 * - Native TypeScript control structures: if, else
 * - Native TypeScript loops: while, for
 * - Action coalescing (merging consecutive actions into single trigger blocks)
 * - Standalone compile via `eudkit build`
 */

import { session, Player1, Add, SetDeaths, SetResources, Ore } from "eudkit";

export let a = session.var(0, "a");
export let b = session.var(20, "b");
export let c = session.var(0, "c");

export function main() {
    // 1. Compile-time loop unrolling with action coalescing
    for (let i = 0; i < 10; i++) {
        a.add(1);
        SetDeaths(Player1, Add, 1, "Terran Marine");
    }

    // 2. Idiomatic if / else branches
    if (a.get() >= 30) {
        c.set(100);
        SetResources(Player1, Add, 500, Ore);
    } else if (a.get() == 15) {
        c.set(50);
    } else {
        c.set(10);
    }

    // 3. Standard while loop
    while (b.get() >= 1) {
        b.subtract(1);
        if (b.get() == 5) {
            a.add(10);
            continue;
        }
        if (a.get() >= 200) {
            break;
        }
        a.add(2);
    }
}
