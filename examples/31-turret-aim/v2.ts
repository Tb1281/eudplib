/**
 * Scenario 31: Fixed Turret Aiming and Range Tracking — Modern eudkit (v2).
 *
 * Ideal Intent:
 * A fixed turret at pixel coordinates (1280, 1280) tracks a moving target (target_x, target_y):
 * 1. Computes coordinate deltas:
 *    dx = |target_x - 1280|
 *    dy = |target_y - 1280|
 * 2. Computes squared Euclidean distance: `dist_sq = dx * dx + dy * dy`.
 *    In modern game engine architecture, comparing squared distance against `RANGE^2`
 *    (384^2 = 147,456) expresses the ideal mathematical intent without invoking an
 *    expensive iterative square-root approximation every frame.
 * 3. If target is within range (`dist_sq <= 147456`) and cooldown has elapsed:
 *    - Kills 1 enemy Zergling at aim zone (location 6) for Player 8 via `KillUnitAt`.
 *    - Records shot count (`shots += 1`).
 *    - Arms cooldown timer (24 ticks).
 * 4. If out of range: increments `out_of_range`.
 * 5. Records `longest_sq` observed (capped at `2000^2 = 4,000,000`).
 */

import {
    KillUnitAt,
    Player8,
    session,
} from "eudkit";

export const TURRET_X = 1280;
export const TURRET_Y = 1280;
export const RANGE = 384;
export const RANGE_SQ = (RANGE * RANGE);
export const FIRE_COOLDOWN = 24;
export const LOC_AIM_ZONE = 6;
export const MAX_TRACK_DIST_SQ = (2000 * 2000);
export let target_x = session.var(1280, "target_x");
export let target_y = session.var(1280, "target_y");
export let dx = session.var(0, "dx");
export let dy = session.var(0, "dy");
export let dist_sq = session.var(0, "dist_sq");
export let cooldown = session.var(0, "cooldown");
export let shots = session.var(0, "shots");
export let out_of_range = session.var(0, "out_of_range");
export let longest_sq = session.var(0, "longest_sq");

export function main(): void {
    if ((target_x >= TURRET_X)) {
        dx = (target_x - TURRET_X);
    } else {
        dx = (TURRET_X - target_x);
    }
    if ((target_y >= TURRET_Y)) {
        dy = (target_y - TURRET_Y);
    } else {
        dy = (TURRET_Y - target_y);
    }
    dist_sq = ((dx * dx) + (dy * dy));
    if (((dist_sq > longest_sq) && (dist_sq <= MAX_TRACK_DIST_SQ))) {
        longest_sq = dist_sq;
    }
    if ((cooldown >= 1)) {
        cooldown -= 1;
    }
    if ((dist_sq <= RANGE_SQ)) {
        if ((cooldown == 0)) {
            KillUnitAt(1, "Zerg Zergling", LOC_AIM_ZONE, Player8);
            shots += 1;
            cooldown = FIRE_COOLDOWN;
        }
    } else {
        out_of_range += 1;
    }
}
