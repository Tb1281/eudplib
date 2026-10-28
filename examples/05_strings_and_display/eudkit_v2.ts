/**
 * Example 05: Strings & Display — Modern eudkit (TypeScript).
 *
 * Demonstrates:
 * - Canonical `session.string(...)` factory for mutable in-game formatted strings
 * - Direct screen overlay `display_at(line, ...)` and chat `DisplayText(...)`
 * - Local player filtering via `IsUserCP()`
 * - Standalone compile via `eudkit build`
 */

import { session, DisplayText, display_at, IsUserCP } from "eudkit";

export let score = session.var(100, "score");
export let minerals = session.var(500, "minerals");
export let gas = session.var(250, "gas");

export let hud = session.string("Score: %d | Min: %d | Gas: %d");

export function main() {
    // 1. Direct formatted chat display
    DisplayText("Status Report: Score = %d", score);

    // 2. Fixed on-screen HUD line display (0..12)
    hud.display_at(0, score, minerals, gas);

    // 3. Local player filtering
    if (IsUserCP()) {
        display_at(1, "Local Player View - Score: %d", score);
    }
}
