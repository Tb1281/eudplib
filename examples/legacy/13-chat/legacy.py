"""13-chat (legacy) — chat detection via 0x58D900 / 0x58D904.

Intent: when a player sends a chat message, the engine leaves the
sender's player id at 0x58D900 and the first message dword at
0x58D904. Every trigger loop: detect "a message is present"
(message dword != 0), capture the sender into a var, count
messages, and light a lamp when the sender is Player 1 (id 0).
Legacy idiom: two f_dwread_epd materializations up front, then
plain var compares gating the bookkeeping — detection itself pays
two full reads every loop, message or not.

Amortization note: f_dwread_epd rides the import-time readtable
engine (~80T once per session, excluded from this delta); only
the two call sites are counted.
"""

from eudkit import *

CHAT_SENDER_EPD = EPD(0x58D900)
CHAT_MSG_EPD = EPD(0x58D904)

last_sender = EUDVariable(0xFF)  # 0xFF = nobody yet
chat_count = EUDVariable(0)
p1_lamp = EUDVariable(0)


def run():
    global last_sender, chat_count, p1_lamp
    # Read both slots every loop; detection is a var compare on the
    # materialized message head.
    sender = f_dwread_epd(CHAT_SENDER_EPD)
    msg_head = f_dwread_epd(CHAT_MSG_EPD)

    if EUDIf()(msg_head != 0):
        last_sender << sender
        chat_count << chat_count + 1

        if EUDIf()(sender == 0):  # Player 1
            p1_lamp << 1
        if EUDElse()():
            p1_lamp << 0
        EUDEndIf()
    EUDEndIf()
