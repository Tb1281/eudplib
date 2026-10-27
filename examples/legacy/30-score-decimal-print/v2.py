"""Scenario 30: Decimal Scoreboard Print — Modern eudkit (v2).

Ideal Intent:
Track Player 1's score from two deaths counters:
- Player 8 Zergling deaths = kills scored (`KILLS_ADDR`)
- Player 1 Marine deaths = deaths suffered (`DEATHS_ADDR`)
Every 120 trigger loops:
1. Score = max(0, kills * 100 - deaths * 30).
2. Accuracy = (kills * 100) // (kills + deaths) if kills + deaths > 0 else 0.
3. Composes and displays formatted decimal status line:
   `"SCORE: %d  KILLS: %d  DEATHS: %d  ACC: %d%%"`
Modern v2 uses native integer floor division `//`, declarative timing `with every(120):`,
and unified `session.string_buffer()` formatting.
"""

from pathlib import Path
import eudkit
from eudkit import (
    EPD,
    dwread_epd,
    every,
)

REPO_ROOT = Path(__file__).resolve().parents[3]
BASEMAP_PATH = REPO_ROOT / "tests" / "outputmap" / "basemap" / "basemap_strx.scx"

# Deaths table addresses: 0x58A364 + 4*player + 48*unit
KILLS_ADDR = 0x58A364 + 4 * 7 + 48 * 37    # Player 8, Zergling
DEATHS_ADDR = 0x58A364                     # Player 1, Marine
PRINT_PERIOD = 120
SCORE_PER_KILL = 100
PENALTY_PER_DEATH = 30


def build(session: eudkit.Session):
    board_buf = session.string_buffer(128)
    prev_kills = session.var(0, name="prev_kills")
    prev_deaths = session.var(0, name="prev_deaths")
    kills_total = session.var(0, name="kills_total")
    deaths_total = session.var(0, name="deaths_total")
    score_base = session.var(0, name="score_base")
    score_penalty = session.var(0, name="score_penalty")
    score = session.var(0, name="score")
    accuracy = session.var(0, name="accuracy")
    prints = session.var(0, name="prints")

    @session.fn
    def game_loop():
        # Accumulate kills
        cur_k = dwread_epd(EPD(KILLS_ADDR))
        if cur_k > prev_kills:
            kills_total += cur_k - prev_kills
        prev_kills = cur_k

        # Accumulate deaths
        cur_d = dwread_epd(EPD(DEATHS_ADDR))
        if cur_d > prev_deaths:
            deaths_total += cur_d - prev_deaths
        prev_deaths = cur_d

        # Score calculation with unsigned floor at 0
        score_base = kills_total * SCORE_PER_KILL
        score_penalty = deaths_total * PENALTY_PER_DEATH
        if score_base >= score_penalty:
            score = score_base - score_penalty
        else:
            score = 0

        # Periodic scoreboard refresh
        with every(PRINT_PERIOD):
            total_events = kills_total + deaths_total
            if total_events >= 1:
                accuracy = (kills_total * 100) // total_events
            else:
                accuracy = 0

            board_buf.printf(
                "SCORE: %d  KILLS: %d  DEATHS: %d  ACC: %d%%",
                score,
                kills_total,
                deaths_total,
                accuracy,
            )
            board_buf.display()
            prints += 1

    return {
        "score": score,
        "kills_total": kills_total,
        "deaths_total": deaths_total,
        "accuracy": accuracy,
        "prints": prints,
    }


def main():
    session = eudkit.Session(BASEMAP_PATH)
    build(session)
    out_path = Path(__file__).parent / "output_v2.scx"
    report = session.compile(out_path)
    print("Scenario 30 (v2) compiled successfully:")
    print(f"  Triggers: {report.trigger_count}")
    print(f"  Payload:  {report.payload_size} bytes")
    print(f"  VProcs:   {report.vproc_count}")
    return report


if __name__ == "__main__":
    main()
