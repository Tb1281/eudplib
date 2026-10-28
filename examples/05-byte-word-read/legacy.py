"""05-byte-word-read (legacy) — sub-dword reads at fixed + cursor addrs.

Intent: every trigger loop, read (a) the low byte and low word of
the dword at 0x58A364+0x40, (b) the unit-type word of the first
CUnit slot, and (c) one byte at a game-time EPD cursor — the values
themselves are needed (stored into variables for later systems).
Legacy idiom: f_bread_epd / f_wread_epd are readtable-based readers;
each assembles the value bit-class by bit-class into a return
variable.

Amortization note: the readtable engine (~80 chain triggers) is
built once per session at import and is NOT in this scenario's
delta — every legacy read below costs only its call site.
"""

from eudkit import *

# 0x58A364 + 0x40: a fixed score-table dword whose sub-fields we poll.
FIXED_EPD = EPD(0x58A364 + 0x40)
# CUnit table head; unit type is the word at byte offset 0x64.
UNIT_TYPE_EPD = EPD(0x59CCA8 + 0x64)

low_byte = EUDVariable(0)
low_word = EUDVariable(0)
unit_type = EUDVariable(0)
cursor_epd = EUDVariable(EPD(0x59CCA8))
cursor_byte = EUDVariable(0)
status = EUDVariable(0)


def run():
    global low_byte, low_word, unit_type, cursor_byte, status
    # (a) sub-dword reads of the fixed dword: byte at sub 0, word at
    # sub 0. Each call site is small; the engine behind it is shared.
    low_byte << f_bread_epd(FIXED_EPD, 0)
    low_word << f_wread_epd(FIXED_EPD, 0)

    # (b) unit-type word: the address is 2-aligned, so sub 0 of the
    # containing dword — f_wread_epd handles the window.
    unit_type << f_wread_epd(UNIT_TYPE_EPD, 0)

    # (c) cursor byte: the cursor holds a game-time EPD and advances
    # by one dword per loop elsewhere; here we only read byte sub 2.
    cursor_byte << f_bread_epd(cursor_epd, 2)

    # Use the values (keep them live): pack the polled fields back
    # into one status dword for a display system downstream.
    status << low_byte + (low_word << 8) + (unit_type << 16)
    if EUDIf()(cursor_byte == 0xFF):
        status << status + 1
    EUDEndIf()
