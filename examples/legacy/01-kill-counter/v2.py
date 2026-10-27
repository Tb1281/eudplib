"""Scenario 01: Kill Counter — Modern eudkit (v2).

Ideal Intent:
An escalating-threshold kill reward system. Every trigger loop, the engine's
Marine deaths counter for Player1 is evaluated against an escalating threshold.
When reached:
- 100 Ore is awarded via typed SetResources
- The deaths counter is reset to 0 via SetDeaths
- Consumed kills are credited to lifetime stats
- The threshold escalates by +15 kills
- Every 3rd payout awards an extra wave bonus of 50 Ore and tracks best wave stats.
"""

from pathlib import Path
import eudkit
from eudkit import (
    Add,
    AtLeast,
    Deaths,
    Ore,
    Player1,
    SetDeaths,
    SetResources,
    SetTo,
    once,
)

REPO_ROOT = Path(__file__).resolve().parent.parent.parent.parent
BASEMAP_PATH = REPO_ROOT / "tests" / "outputmap" / "basemap" / "basemap_strx.scx"


def build(session: eudkit.Session):
    # State tracking
    kills_credited = session.var(0, name="kills_credited")
    next_reward = session.var(10, name="next_reward")
    rewards_paid = session.var(0, name="rewards_paid")
    rewards_this_wave = session.var(0, name="rewards_this_wave")
    ore_paid_total = session.var(0, name="ore_paid_total")
    best_wave = session.var(0, name="best_wave")

    @session.fn
    def game_loop():
        # One-time initialization guard using modern reactive once() context
        with once():
            next_reward = 10
            rewards_this_wave = 0

        # Dynamic threshold condition evaluated directly in engine memory
        if Deaths(Player1, AtLeast, next_reward, "Terran Marine"):
            # High-level typed resource reward (no raw memory offset math)
            SetResources(Player1, Add, 100, Ore)
            ore_paid_total += 100

            # Consume the counter
            SetDeaths(Player1, SetTo, 0, "Terran Marine")

            # Update bookkeeping stats
            kills_credited += next_reward
            next_reward += 15
            rewards_paid += 1
            rewards_this_wave += 1

            # Wave bonus milestone check
            if rewards_this_wave >= 3:
                SetResources(Player1, Add, 50, Ore)
                ore_paid_total += 50
                if rewards_this_wave > best_wave:
                    best_wave = rewards_this_wave
                rewards_this_wave = 0

    return {
        "kills_credited": kills_credited,
        "next_reward": next_reward,
        "rewards_paid": rewards_paid,
        "rewards_this_wave": rewards_this_wave,
        "ore_paid_total": ore_paid_total,
        "best_wave": best_wave,
    }


def main():
    session = eudkit.Session(BASEMAP_PATH)
    build(session)
    out_path = Path(__file__).parent / "output_v2.scx"
    report = session.compile(out_path)
    print("Scenario 01 (v2) compiled successfully:")
    print(f"  Triggers: {report.trigger_count}")
    print(f"  Payload:  {report.payload_size} bytes")
    print(f"  VProcs:   {report.vproc_count}")
    return report


if __name__ == "__main__":
    main()
