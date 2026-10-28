"""Scenario 13: In-Game Chat Detection — Modern eudkit (v2).

Ideal Intent:
Detect and process in-game chat events from StarCraft engine memory (`0x58D900` / `0x58D904`):
1. Detect chat message arrival (`CHAT_MSG_EPD != 0`).
2. Lazily read sender ID (`CHAT_SENDER_EPD`) only when a message is actually present.
3. Record `last_sender`, increment `chat_count`.
4. Illuminate `p1_lamp` if the sender is Player 1 (`sender == 0`), else turn it off.
"""

import eudkit
from eudkit import (
    session,
    EPD,
    dwread_epd,
)


CHAT_SENDER_EPD = EPD(0x58D900)
CHAT_MSG_EPD = EPD(0x58D904)

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
