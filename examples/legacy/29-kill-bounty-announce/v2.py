"""Scenario 29: Kill Bounty and Broadcast Announcement — Modern eudkit (v2).

Ideal Intent:
Monitors Player 1 Marine deaths counter:
1. When counter increases, compute delta `new_kills = cur_kills - prev_kills`.
2. Compute bounty: 15 ore per kill.
3. If `new_kills >= 5` (jackpot), add a flat 50-ore bonus and increment `jackpots`.
4. Pay ore by updating Player 1's ore dword at `0x57F0F0`.
5. Formats and displays an in-game broadcast message using modern StringBuffer:
   `"Player 1 scored %d kills! Bounty: %d ore"`
6. Track cumulative statistics: `total_kills_paid`, `total_bounty`, `announcements`.
"""

from pathlib import Path
import eudkit
from eudkit import (
    EPD,
    Add,
    SetDeaths,
    dwread_epd,
)

REPO_ROOT = Path(__file__).resolve().parents[3]
BASEMAP_PATH = REPO_ROOT / "tests" / "outputmap" / "basemap" / "basemap_strx.scx"

MARINE_DEATHS_P1 = 0x58A364
ORE_ADDR_P1 = 0x57F0F0
BOUNTY_PER_KILL = 15
JACKPOT_MIN_KILLS = 5
JACKPOT_BONUS = 50


def build(session: eudkit.Session):
    announce_buf = session.string_buffer(128)
    prev_kills = session.var(0, name="prev_kills")
    cur_kills = session.var(0, name="cur_kills")
    new_kills = session.var(0, name="new_kills")
    bounty = session.var(0, name="bounty")
    total_kills_paid = session.var(0, name="total_kills_paid")
    total_bounty = session.var(0, name="total_bounty")
    jackpots = session.var(0, name="jackpots")
    announcements = session.var(0, name="announcements")

    @session.fn
    def game_loop():
        # Read current marine deaths count using ReadTable lowering
        cur_kills = dwread_epd(EPD(MARINE_DEATHS_P1))

        if cur_kills > prev_kills:
            new_kills = cur_kills - prev_kills
            bounty = new_kills * BOUNTY_PER_KILL

            # Jackpot tier for large batches
            if new_kills >= JACKPOT_MIN_KILLS:
                bounty += JACKPOT_BONUS
                jackpots += 1

            # Disburse bounty ore
            SetDeaths(EPD(ORE_ADDR_P1), Add, bounty, 0)
            total_kills_paid += new_kills
            total_bounty += bounty

            # Format and display announcement via unified StringBuffer
            announce_buf.printf("Player 1 scored %d kills! Bounty: %d ore", new_kills, bounty)
            announce_buf.display()
            announcements += 1

        prev_kills = cur_kills

    return {
        "total_kills_paid": total_kills_paid,
        "total_bounty": total_bounty,
        "jackpots": jackpots,
        "announcements": announcements,
    }


def main():
    session = eudkit.Session(BASEMAP_PATH)
    build(session)
    out_path = Path(__file__).parent / "output_v2.scx"
    report = session.compile(out_path)
    print("Scenario 29 (v2) compiled successfully:")
    print(f"  Triggers: {report.trigger_count}")
    print(f"  Payload:  {report.payload_size} bytes")
    print(f"  VProcs:   {report.vproc_count}")
    return report


if __name__ == "__main__":
    main()
