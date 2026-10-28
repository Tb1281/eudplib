"""Scenario 22: Unit Type Renaming (TBL Patching) — Modern eudkit (v2).

Ideal Intent:
Rename 8 core combat units at game start via StarCraft's TBL string table:
- Terran Marine (TBL id 1) -> "Space Marine"
- Terran Ghost (TBL id 2) -> "Shadow Operative"
- Terran Firebat (TBL id 33) -> "Pyro Specialist"
- Terran Medic (TBL id 35) -> "Field Surgeon"
- Zerg Zergling (TBL id 38) -> "Voidling"
- Zerg Hydralisk (TBL id 39) -> "Spine Beast"
- Protoss Zealot (TBL id 66) -> "Blade Adept"
- Protoss Dragoon (TBL id 67) -> "War Strider"
Pack strings into 4-byte little-endian dwords and emit writes once on map initialization.
"""

import eudkit
from eudkit import (
    session,
    EPD,
    dwwrite_epd,
    epdread_epd,
    once,
    wread_epd,
)


# Core unit renaming roster: (tbl_id, display_name)
ROSTER_RENAMES = [
    (1, "Space Marine"),
    (2, "Shadow Operative"),
    (33, "Pyro Specialist"),
    (35, "Field Surgeon"),
    (38, "Voidling"),
    (39, "Spine Beast"),
    (66, "Blade Adept"),
    (67, "War Strider"),
]

# Build-time packing into 4-byte dword sequences
PACKED_RENAMES = []
for tid, name in ROSTER_RENAMES:
    b = name.encode("utf-8") + b"\x00"
    pad = (4 - len(b) % 4) % 4
    b += b"\x00" * pad
    dwords = [int.from_bytes(b[i : i + 4], "little") for i in range(0, len(b), 4)]
    PACKED_RENAMES.append((tid, dwords))

tbl_epd = session.var(0, name="tbl_epd")
str_offset = session.var(0, name="str_offset")
dst_epd = session.var(0, name="dst_epd")
renamed = session.var(0, name="renamed")

@session.fn
def game_loop():
    # Executes exactly once on map startup
    with once():
        # Resolve stat_txt.tbl base pointer at 0x6D5A30
        tbl_epd = epdread_epd(EPD(0x6D5A30))

        for i in range(len(PACKED_RENAMES)):
            entry = PACKED_RENAMES[i]
            # Read 16-bit offset from table header
            str_offset = wread_epd(tbl_epd + (entry[0] // 2), (entry[0] % 2))
            dst_epd = tbl_epd + (str_offset // 4)

            # Write packed 4-byte character blocks
            for j in range(len(entry[1])):
                dwwrite_epd(dst_epd + j, entry[1][j])

        renamed = 1
