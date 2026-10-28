/**
 * Example 01: Variables & Arithmetic — Modern eudkit (TypeScript).
 *
 * Demonstrates:
 * - Unified variable declaration (automatic storage lowering: 4B memory cell vs 72B VProc)
 * - Natural arithmetic expressions (+, -, *, &, |, ^)
 * - Direct action emission without DoActions boilerplate
 * - Standalone compile via `eudkit build --entry eudkit_v2.ts`
 */

import { session, Player1, SetDeaths, SetTo } from "eudkit";

// 1. Declare variables at module level
export let a = session.var(10, "a");
export let b = session.var(20, "b");
export let c = session.var(5, "c");
export let d = session.var(0, "d");

// 2. Define game loop routine using standard TypeScript syntax
export function main() {
    d.set((a.get() + b.get()) - c.get() + (b.get() & 0xFF));
    SetDeaths(Player1, SetTo, d, "Terran Marine");
}
