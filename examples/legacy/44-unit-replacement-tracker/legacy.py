"""LEGACY unit replacement / transformation tracker (benchmark 44).

Same intent as ours.js: watch every CUnit slot for "hero" Marines
(Terran Marine, hp >= 500). When a tracked hero dies, count it. When
a slot that held a dead hero is recycled by a NEW unit instance,
count a replacement. A second pass recomputes the alive count.

Legacy has no tracker primitive, so the map keeps per-slot shadow
state by hand: one EUDArray for the slot's LIVE/DEAD state and one
for the unit's identity (its sprite pointer value acts as an
instance id). Every frame all 1700 slots are scanned and the shadow
is diffed:

  state LIVE, sprite == 0            -> the hero died
  state DEAD, sprite != 0, hero      -> the slot was replaced
  state LIVE, identity changed       -> died AND replaced unseen

Sprite/hp/unitType reads all go through CUnit member reads (dwread):
the loop hands you values, never conditions.
"""

from eudkit import *
from eudkit.scdata import CUnit

CUNIT_BASE = 0x59CCA8
CUNIT_STRIDE = 336
CUNIT_SLOTS = 1700
MARINE = 0
HERO_HP = 500

EMPTY, LIVE, DEAD = 0, 1, 2

shadow_state = EUDArray(CUNIT_SLOTS)
shadow_ident = EUDArray(CUNIT_SLOTS)

slot = EUDVariable(0)
heroes_died = EUDVariable(0)
heroes_replaced = EUDVariable(0)
heroes_alive = EUDVariable(0)
alive_scan = EUDVariable(0)


def run():
    global slot, heroes_died, heroes_replaced, heroes_alive, alive_scan
    if EUDWhile()(slot < CUNIT_SLOTS):
        unit = CUnit.from_ptr(CUNIT_BASE + CUNIT_STRIDE * slot)
        sprite = unit.sprite
        ident = sprite  # sprite pointer value: unique per unit instance

        if EUDIf()(sprite == 0):
            # Slot is empty now.
            if EUDIf()(shadow_state[slot] == LIVE):
                heroes_died += 1
                shadow_state[slot] = DEAD
            EUDEndIf()
        EUDEndIf()

        if EUDIf()(sprite != 0):
            if EUDIf()(shadow_state[slot] == DEAD):
                # A new instance occupies a slot whose hero died earlier.
                if EUDIf()(unit.unitType == MARINE):
                    if EUDIf()(unit.hp >= HERO_HP):
                        heroes_replaced += 1
                        shadow_state[slot] = LIVE
                        shadow_ident[slot] = ident
                    EUDEndIf()
                EUDEndIf()
            EUDEndIf()
            if EUDIf()(shadow_state[slot] == LIVE):
                if EUDIf()(shadow_ident[slot] != ident):
                    # Same slot, different instance: died + replaced unseen.
                    heroes_died += 1
                    heroes_replaced += 1
                    shadow_ident[slot] = ident
                EUDEndIf()
            EUDEndIf()
            if EUDIf()(shadow_state[slot] == EMPTY):
                if EUDIf()(unit.unitType == MARINE):
                    if EUDIf()(unit.hp >= HERO_HP):
                        shadow_state[slot] = LIVE
                        shadow_ident[slot] = ident
                    EUDEndIf()
                EUDEndIf()
            EUDEndIf()
        EUDEndIf()

        slot += 1
    EUDEndWhile()
    slot << 0

    # Per-frame bookkeeping: recompute the alive count from the shadow.
    heroes_alive << 0
    if EUDWhile()(alive_scan < CUNIT_SLOTS):
        if EUDIf()(shadow_state[alive_scan] == LIVE):
            heroes_alive += 1
        EUDEndIf()
        alive_scan += 1
    EUDEndWhile()
    alive_scan << 0
