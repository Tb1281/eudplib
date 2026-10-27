"""12-nickname (legacy) — read a player's nickname bytes (0x57EEE8).

Intent: the nickname table starts at 0x57EEE8; each player record
is 36 bytes and the name bytes begin at record+3 (player 0's name at
0x57EEEB, cf. eudplib's own pname machinery). For the player index
in a var, read the first 4 name bytes every trigger loop; raise a
flag when the name is non-empty and starts with 'A' (0x41), and fold
the bytes into a checksum var. Legacy idiom (the edac pattern):
TWO EUDByteReaders — re-seeking one reader costs a fresh div/mod
seek, so two readers alternate over the bytes.

Amortization note: both readers ride the import-time readtable
engine (~80T once per session, excluded from this delta); the
readers' own seek/read machinery is built at construction (module
level here), also outside the delta — only the in-run() call sites
and seeks are counted.
"""

from eudkit import *

NAME_TABLE = 0x57EEE8
RECORD_STRIDE = 36  # bytes per player record
NAME_OFFSET = 3  # name bytes start at record + 3

target_player = EUDVariable(1)
name_flag = EUDVariable(0)
name_sum = EUDVariable(0)
addr = EUDVariable(0)

br_record = EUDByteReader()
br_name = EUDByteReader()


def run():
    global name_flag, name_sum, addr
    # Byte address of this player's name: table + 36*player + 3.
    addr << NAME_TABLE + RECORD_STRIDE * target_player + NAME_OFFSET

    # Two readers alternate so neither pays a second seek: record
    # reader takes bytes 0 and 2, name reader takes 1 and 3.
    br_record.seekoffset(addr)
    br_name.seekoffset(addr + 1)
    b0 = br_record.readbyte()
    b1 = br_name.readbyte()
    b2 = br_record.readbyte()
    b3 = br_name.readbyte()

    name_sum << b0 + (b1 << 8) + (b2 << 16) + (b3 << 24)

    if EUDIf()([b0 != 0, b0 == 0x41]):
        name_flag << 1
    if EUDElse()():
        name_flag << 0
    EUDEndIf()
