/**
 * Example 03: Functions & Inlining — Modern eudkit (TypeScript).
 *
 * Demonstrates:
 * - Small helper function inlining
 * - Direct-return calling convention
 * - Standalone compile via `eudkit build`
 */

import { session } from "eudkit";

export let r1 = session.var(0, "r1");
export let val = session.var(0, "val");

function add3(x: number, y: number, z: number): number {
    return x + y + z;
}

function worker(x: number): number {
    return x + 7;
}

export function main() {
    // Inlined helper
    val.set(add3(5, 15, 10));

    // Function call with direct return
    r1.set(worker(val.get()));
}
