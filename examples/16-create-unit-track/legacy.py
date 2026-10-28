"""Scenario 16 (legacy) — periodic unit creation + track the new unit.

Intent: every 24 trigger loops, create one Terran Marine for Player 1
at location 1; BEFORE creating, capture the "next unit" slot (the
CUnit the engine is about to fill), then set the created unit's hp to
512, remember its pointer, and count total spawns.
Legacy idiom: f_dwread(0x628438) + CUnit.from_ptr — the capture must
happen before the CreateUnit action fires, in the same trigger.
"""

from eudkit import *  # noqa: F403

NEXT_UNIT_PTR_ADDR = 0x628438  # "First Empty Unit": slot of next creation
MARINE = 0  # Terran Marine unit id
SPAWN_LOCATION = 1  # map location index (id form: no name lookup)
SPAWN_PERIOD = 24  # loops between spawns

tick = EUDVariable()
spawn_count = EUDVariable()
last_spawn_ptr = EUDVariable()
last_spawn_hp = EUDVariable()


def run():
    global tick, spawn_count, last_spawn_ptr, last_spawn_hp
    tick += 1
    if EUDIf()(tick >= SPAWN_PERIOD):
        tick << 0

        # 1) Capture first: read the next-unit pointer BEFORE the
        #    CreateUnit below fills that slot.
        next_ptr = f_dwread(NEXT_UNIT_PTR_ADDR)
        new_unit = CUnit.from_ptr(next_ptr)

        # 2) Create the unit (engine action, 1 trigger action).
        DoActions([CreateUnit(1, MARINE, SPAWN_LOCATION, Player1)])

        # 3) The captured handle now IS the created unit: buff its hp
        #    and record identity for later logic.
        new_unit.hp << 512
        last_spawn_ptr << next_ptr
        spawn_count += 1
        last_spawn_hp << new_unit.hp
    EUDEndIf()

    # Milestone flag every 10 spawns (string-free, pilot convention).
    if EUDIf()(spawn_count >= 10):
        DoActions([SetDeaths(Player1, SetTo, 1, 204)])
    EUDEndIf()
