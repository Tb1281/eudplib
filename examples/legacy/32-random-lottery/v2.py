"""Scenario 32: Random Lottery and Prize Tier Distribution — Modern eudkit (v2).

Ideal Intent:
Every 200 trigger loops, each of 4 active players (Players 1 to 4) draws a lottery roll (0..99):
1. Prize tiers:
   - Roll < 5: 500 Ore (Jackpot tier, increments jackpots counter)
   - Roll < 25: 100 Ore (Big tier)
   - Roll < 60: 25 Ore (Small tier)
   - Roll >= 60: No prize
2. Track total draws (`draws`), total ore disbursed (`total_paid`), and lowest roll seen (`best_roll`).
Modern v2 uses:
- Declarative timing (`with every(200):`).
- Inlined fast LCG pseudo-random generator eliminating external EUDFunc trampolines.
- Direct constant initialization (`best_roll = session.var(100)`) eliminating legacy `seen_any` guard flags.
"""

from pathlib import Path
import eudkit
from eudkit import (
    Add,
    Ore,
    SetResources,
    every,
)

REPO_ROOT = Path(__file__).resolve().parents[3]
BASEMAP_PATH = REPO_ROOT / "tests" / "outputmap" / "basemap" / "basemap_strx.scx"

LOTTERY_PERIOD = 200
JACKPOT_ROLL = 5
BIG_ROLL = 25
SMALL_ROLL = 60
JACKPOT_ORE = 500
BIG_ORE = 100
SMALL_ORE = 25
PLAYERS_COUNT = 4


def build(session: eudkit.Session):
    seed = session.var(1234567, name="prng_seed")
    draws = session.var(0, name="draws")
    jackpots = session.var(0, name="jackpots")
    total_paid = session.var(0, name="total_paid")
    best_roll = session.var(100, name="best_roll")  # 100 is higher than max roll (99)
    roll = session.var(0, name="roll")

    @session.fn
    def game_loop():
        nonlocal seed, roll, draws, jackpots, total_paid, best_roll
        with every(LOTTERY_PERIOD):
            # Unrolled per-player draw loop
            for p in range(PLAYERS_COUNT):
                # Linear Congruential PRNG step (Numerical Recipes constants)
                seed = seed * 1664525 + 1013904223
                roll = seed % 100
                draws += 1

                # Prize distribution tiers
                if roll < JACKPOT_ROLL:
                    SetResources(p, Add, JACKPOT_ORE, Ore)
                    total_paid += JACKPOT_ORE
                    jackpots += 1
                elif roll < BIG_ROLL:
                    SetResources(p, Add, BIG_ORE, Ore)
                    total_paid += BIG_ORE
                elif roll < SMALL_ROLL:
                    SetResources(p, Add, SMALL_ORE, Ore)
                    total_paid += SMALL_ORE

                # Track best (lowest) roll directly
                if roll < best_roll:
                    best_roll = roll

    return {
        "draws": draws,
        "jackpots": jackpots,
        "total_paid": total_paid,
        "best_roll": best_roll,
    }


def main():
    session = eudkit.Session(BASEMAP_PATH)
    build(session)
    out_path = Path(__file__).parent / "output_v2.scx"
    report = session.compile(out_path)
    print("Scenario 32 (v2) compiled successfully:")
    print(f"  Triggers: {report.trigger_count}")
    print(f"  Payload:  {report.payload_size} bytes")
    print(f"  VProcs:   {report.vproc_count}")
    return report


if __name__ == "__main__":
    main()
