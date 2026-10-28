/**
 * Scenario 04: Arithmetic Expression Chain — Modern eudkit (v2).
 *
 * Ideal Intent:
 * Calculate complex compound arithmetic expressions over game state variables:
 * 1. Attack calculation: `((atk_base + atk_bonus) * atk_mul - armor_flat) // divisor % wrap_mod`
 * 2. Cooldown timing: unpack high/low words from raw timer with bitwise shift and mask,
 *    derive phase ticks using integer division (`//`) and modulo (`%`).
 * 3. Bounded accumulation loop: compute unrolled linear summation.
 * 4. Threshold check: reset result when overflowing threshold.
 */

import {
    session,
} from "eudkit";

export let atk_base = session.var(120, "atk_base");
export let atk_bonus = session.var(35, "atk_bonus");
export let atk_mul = session.var(3, "atk_mul");
export let armor_flat = session.var(17, "armor_flat");
export let divisor = session.var(4, "divisor");
export let wrap_mod = session.var(1000, "wrap_mod");
export let result = session.var(0, "result");
export let raw_timer = session.var(0x12345678, "raw_timer");
export let cd_ticks = session.var(0, "cd_ticks");
export let cd_phase = session.var(0, "cd_phase");
export let loop_guard = session.var(0, "loop_guard");
export let hi = session.var(0, "hi");

export function main(): void {
    result = ((((((atk_base + atk_bonus) * atk_mul) - armor_flat) / divisor) | 0) % wrap_mod);
    hi = (raw_timer >> 16);
    cd_ticks = (((hi * 7) + (raw_timer & 65535)) % 3600);
    cd_phase = ((((cd_ticks / 60) | 0) & 255) ^ (cd_ticks % 60));
    loop_guard = 0;
    while ((loop_guard < 24)) {
        result += (loop_guard * 3);
        loop_guard += 1;
    }
    if ((result >= 500)) {
        result = 0;
    }
}
