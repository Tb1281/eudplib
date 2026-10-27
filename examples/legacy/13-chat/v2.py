"""Scenario 13: In-Game Chat Detection — Modern eudkit (v2).

Ideal Intent:
Detect and process in-game chat events from StarCraft engine memory (`0x58D900` / `0x58D904`):
1. Detect chat message arrival (`CHAT_MSG_EPD != 0`).
2. Lazily read sender ID (`CHAT_SENDER_EPD`) only when a message is actually present.
3. Record `last_sender`, increment `chat_count`.
4. Illuminate `p1_lamp` if the sender is Player 1 (`sender == 0`), else turn it off.
"""

from pathlib import Path
import eudkit
from eudkit import (
    EPD,
    dwread_epd,
)

REPO_ROOT = Path(__file__).resolve().parents[3]
BASEMAP_PATH = REPO_ROOT / "tests" / "outputmap" / "basemap" / "basemap_strx.scx"

CHAT_SENDER_EPD = EPD(0x58D900)
CHAT_MSG_EPD = EPD(0x58D904)


def build(session: eudkit.Session):
    last_sender = session.var(0xFF, name="last_sender")
    chat_count = session.var(0, name="chat_count")
    p1_lamp = session.var(0, name="p1_lamp")
    sender = session.var(0, name="sender")

    @session.fn
    def game_loop():
        # Lazy detection: only read sender if a message exists
        if dwread_epd(CHAT_MSG_EPD) != 0:
            sender = dwread_epd(CHAT_SENDER_EPD)
            last_sender = sender
            chat_count += 1

            if sender == 0:  # Player 1
                p1_lamp = 1
            else:
                p1_lamp = 0

    return {
        "last_sender": last_sender,
        "chat_count": chat_count,
        "p1_lamp": p1_lamp,
        "sender": sender,
    }


def main():
    session = eudkit.Session(BASEMAP_PATH)
    build(session)
    out_path = Path(__file__).parent / "output_v2.scx"
    report = session.compile(out_path)
    print("Scenario 13 (v2) compiled successfully:")
    print(f"  Triggers: {report.trigger_count}")
    print(f"  Payload:  {report.payload_size} bytes")
    print(f"  VProcs:   {report.vproc_count}")
    return report


if __name__ == "__main__":
    main()
