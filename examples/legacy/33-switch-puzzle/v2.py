"""Scenario 33: Switch Toggle Puzzle and Near-Miss Hints — Modern eudkit (v2).

Ideal Intent:
A puzzle door guarded by 4 switches (switches 0..3):
- Correct combination: switch 0 Set, switch 1 Cleared, switch 2 Set, switch 3 Set.
- Each loop, re-evaluate matching switch count (`match_count`).
- When match_count == 4 (solved edge):
  - Award 300 Ore to Player 1, ping door location (location 7), increment `solves`.
  - Latches `solved_latch = 1` until combination breaks (`match_count <= 3`).
- When match_count == 3 (near-miss):
  - Increment `near_miss_loops`.
  - Throttled hint ping: if `hint_cd == 0`, ping door, increment `hints`, reset cooldown to 240.
"""

from pathlib import Path
import eudkit
from eudkit import (
    Add,
    Cleared,
    MinimapPing,
    Ore,
    Player1,
    Set,
    SetResources,
    Switch,
)

REPO_ROOT = Path(__file__).resolve().parents[3]
BASEMAP_PATH = REPO_ROOT / "tests" / "outputmap" / "basemap" / "basemap_strx.scx"

SW_A, SW_B, SW_C, SW_D = 0, 1, 2, 3
LOC_DOOR = 7
SOLVE_ORE = 300
HINT_COOLDOWN = 240


def build(session: eudkit.Session):
    match_count = session.var(0, name="match_count")
    solved_latch = session.var(0, name="solved_latch")
    solves = session.var(0, name="solves")
    near_miss_loops = session.var(0, name="near_miss_loops")
    hints = session.var(0, name="hints")
    hint_cd = session.var(0, name="hint_cd")

    @session.fn
    def game_loop():
        # Hint cooldown countdown
        if hint_cd >= 1:
            hint_cd -= 1

        # Re-score combination
        match_count = 0
        if Switch(SW_A, Set):
            match_count += 1
        if Switch(SW_B, Cleared):
            match_count += 1
        if Switch(SW_C, Set):
            match_count += 1
        if Switch(SW_D, Set):
            match_count += 1

        # Solved edge transition
        if (match_count >= 4) and (solved_latch == 0):
            SetResources(Player1, Add, SOLVE_ORE, Ore)
            MinimapPing(LOC_DOOR)
            solves += 1
            solved_latch = 1

        if (match_count <= 3) and (solved_latch == 1):
            solved_latch = 0

        # Near-miss hint (throttled)
        if match_count == 3:
            near_miss_loops += 1
            if hint_cd == 0:
                MinimapPing(LOC_DOOR)
                hints += 1
                hint_cd = HINT_COOLDOWN

    return {
        "match_count": match_count,
        "solves": solves,
        "near_miss_loops": near_miss_loops,
        "hints": hints,
    }


def main():
    session = eudkit.Session(BASEMAP_PATH)
    build(session)
    out_path = Path(__file__).parent / "output_v2.scx"
    report = session.compile(out_path)
    print("Scenario 33 (v2) compiled successfully:")
    print(f"  Triggers: {report.trigger_count}")
    print(f"  Payload:  {report.payload_size} bytes")
    print(f"  VProcs:   {report.vproc_count}")
    return report


if __name__ == "__main__":
    main()
