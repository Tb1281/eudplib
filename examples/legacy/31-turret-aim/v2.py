"""Scenario 31: Fixed Turret Aiming and Range Tracking — Modern eudkit (v2).

Ideal Intent:
A fixed turret at pixel coordinates (1280, 1280) tracks a moving target (target_x, target_y):
1. Computes coordinate deltas:
   dx = |target_x - 1280|
   dy = |target_y - 1280|
2. Computes squared Euclidean distance: `dist_sq = dx * dx + dy * dy`.
   In modern game engine architecture, comparing squared distance against `RANGE^2`
   (384^2 = 147,456) expresses the ideal mathematical intent without invoking an
   expensive iterative square-root approximation every frame.
3. If target is within range (`dist_sq <= 147456`) and cooldown has elapsed:
   - Kills 1 enemy Zergling at aim zone (location 6) for Player 8 via `KillUnitAt`.
   - Records shot count (`shots += 1`).
   - Arms cooldown timer (24 ticks).
4. If out of range: increments `out_of_range`.
5. Records `longest_sq` observed (capped at `2000^2 = 4,000,000`).
"""

from pathlib import Path
import eudkit
from eudkit import (
    KillUnitAt,
    Player8,
)

REPO_ROOT = Path(__file__).resolve().parents[3]
BASEMAP_PATH = REPO_ROOT / "tests" / "outputmap" / "basemap" / "basemap_strx.scx"

TURRET_X = 1280
TURRET_Y = 1280
RANGE = 384
RANGE_SQ = RANGE * RANGE  # 147,456
FIRE_COOLDOWN = 24
LOC_AIM_ZONE = 6
MAX_TRACK_DIST_SQ = 2000 * 2000  # 4,000,000


def build(session: eudkit.Session):
    target_x = session.var(1280, name="target_x")
    target_y = session.var(1280, name="target_y")
    dx = session.var(0, name="dx")
    dy = session.var(0, name="dy")
    dist_sq = session.var(0, name="dist_sq")
    cooldown = session.var(0, name="cooldown")
    shots = session.var(0, name="shots")
    out_of_range = session.var(0, name="out_of_range")
    longest_sq = session.var(0, name="longest_sq")

    @session.fn
    def game_loop():
        # Absolute coordinate deltas
        if target_x >= TURRET_X:
            dx = target_x - TURRET_X
        else:
            dx = TURRET_X - target_x

        if target_y >= TURRET_Y:
            dy = target_y - TURRET_Y
        else:
            dy = TURRET_Y - target_y

        # Exact squared Euclidean distance
        dist_sq = dx * dx + dy * dy

        # Longest bounded distance tracking
        if (dist_sq > longest_sq) and (dist_sq <= MAX_TRACK_DIST_SQ):
            longest_sq = dist_sq

        # Cooldown timer decrement
        if cooldown >= 1:
            cooldown -= 1

        # Target acquisition and firing decision
        if dist_sq <= RANGE_SQ:
            if cooldown == 0:
                KillUnitAt(1, "Zerg Zergling", LOC_AIM_ZONE, Player8)
                shots += 1
                cooldown = FIRE_COOLDOWN
        else:
            out_of_range += 1

    return {
        "target_x": target_x,
        "target_y": target_y,
        "shots": shots,
        "out_of_range": out_of_range,
        "longest_sq": longest_sq,
    }


def main():
    session = eudkit.Session(BASEMAP_PATH)
    build(session)
    out_path = Path(__file__).parent / "output_v2.scx"
    report = session.compile(out_path)
    print("Scenario 31 (v2) compiled successfully:")
    print(f"  Triggers: {report.trigger_count}")
    print(f"  Payload:  {report.payload_size} bytes")
    print(f"  VProcs:   {report.vproc_count}")
    return report


if __name__ == "__main__":
    main()
