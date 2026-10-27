"""Scenario 18 (legacy) — census three of the engine's five unit lists.

Intent: every frame, count (U) live-list units, (T) "hidden" units —
valid units NOT reachable from the live list — and (S) scanner sweep
units (type 33), plus how many hidden units are Marines.
Legacy has NO API for the hidden list: the mapmaker hand-rolls the
two-phase mark scheme (mark live slots by generation, then scan all
1700 slots for valid-but-unmarked ones) with raw primitives.
"""

from eudkit import *  # noqa: F403

CUNIT_TABLE = 0x59CCA8
CUNIT_STRIDE = 336
SCANNER_SWEEP = 33  # Scanner Sweep unit id
MARINE = 0  # Terran Marine unit id
NONE_TYPE = 228  # "None" unit id: empty/dead slot
UNIT_TYPE_EPD_OFFSET = 0x64 // 4  # unitType word at +0x64

live_count = EUDVariable()
hidden_count = EUDVariable()
hidden_marines = EUDVariable()
scanner_count = EUDVariable()
mark_gen = EUDVariable()
mark = EUDArray(1700)  # one mark dword per CUnit slot


def run():
    global live_count, hidden_count, hidden_marines, scanner_count
    global mark_gen
    live_count << 0
    hidden_count << 0
    hidden_marines << 0
    scanner_count << 0
    mark_gen += 1  # new generation: stale marks never match

    # ---- Phase 1: walk the live list, marking each visited slot ----
    for ptr, epd in EUDLoopUnit():
        live_count += 1
        slot = (ptr - CUNIT_TABLE) // CUNIT_STRIDE
        mark[slot] << mark_gen

    # ---- Phase 2: scan all 1700 slots by hand ----
    slot_idx = EUDVariable()
    slot_idx << 0
    EUDWhile()(slot_idx < 1700)
    slot_epd = EPD(CUNIT_TABLE) + slot_idx * (CUNIT_STRIDE // 4)
    unit_type = f_wread_epd(slot_epd + UNIT_TYPE_EPD_OFFSET, 0)
    if EUDIf()(unit_type != NONE_TYPE):
        if EUDIf()(unit_type == SCANNER_SWEEP):
            scanner_count += 1
        EUDEndIf()
        # Hidden = valid slot whose mark is not this generation's.
        if EUDIf()(mark[slot_idx] != mark_gen):
            hidden_count += 1
            if EUDIf()(unit_type == MARINE):
                hidden_marines += 1
            EUDEndIf()
        EUDEndIf()
    EUDEndIf()
    slot_idx += 1
    EUDEndWhile()
