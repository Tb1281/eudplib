"""09-struct-array (legacy) — struct-array element field access.

Intent: a spawn table of 8 records with the CUnit layout stride
(84 dwords). Every trigger loop: (1) find the first record with
hp == 0 (the free-slot test) and respawn it (hp = 1000), then stop;
(2) tick every live record's energy down by 1. Legacy has no struct
type: records are a raw Db blob and field access is hand-computed
EPD arithmetic — base_epd + i * 84 + field_dword_offset, with
f_dwread_epd / f_dwwrite_epd doing the field I/O (hp at dword 2 =
byte offset 8, energy at dword 48).

Amortization note: f_dwread_epd rides the import-time readtable
engine (~80T once per session, excluded from this delta).
Static vs executed: loop bodies fire up to 8 times per trigger loop.
"""

from eudkit import *

STRIDE = 84  # dwords per record (CUnit-sized)
DW_HP = 2  # hp: byte offset 8
DW_ENERGY = 48  # energy: byte offset 192

# Raw backing blob: 8 records x 336 bytes, all zero (all slots free).
records = Db(b"\x00" * (8 * STRIDE * 4))
rec_epd = EPD(records)

idx = EUDVariable(0)
respawned = EUDVariable(0)


def run():
    global idx, respawned
    # Pass 1: first free slot (hp == 0) -> respawn with full hp,
    # then stop (one respawn per loop).
    idx << 0
    if EUDWhile()(idx < 8):
        slot_epd = rec_epd + idx * STRIDE
        hp = f_dwread_epd(slot_epd + DW_HP)
        if EUDIf()(hp == 0):
            f_dwwrite_epd(slot_epd + DW_HP, 1000)
            respawned << idx + 1  # 1-based marker; 0 = none this loop
            EUDBreak()
        EUDEndIf()
        idx << idx + 1
    EUDEndWhile()

    # Pass 2: energy drain for live records.
    idx << 0
    if EUDWhile()(idx < 8):
        slot_epd = rec_epd + idx * STRIDE
        hp = f_dwread_epd(slot_epd + DW_HP)
        energy = f_dwread_epd(slot_epd + DW_ENERGY)
        if EUDIf()([hp != 0, energy >= 1]):
            f_dwwrite_epd(slot_epd + DW_ENERGY, energy - 1)
        EUDEndIf()
        idx << idx + 1
    EUDEndWhile()
