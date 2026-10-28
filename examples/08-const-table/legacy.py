"""08-const-table (legacy) — const lookup tables (damage-by-level).

Intent: a 32-entry build-time table maps the current level var to
a damage value; every trigger loop, look the value up and feed the
combat system, plus test "damage >= threshold" for a warning lamp,
plus peek the NEXT level's damage against 2x the current one.
Legacy idiom: EUDArray over EUDArrayData — the values live in a
Db-backed data object; var-index reads go through the object's EPD,
and both tests are plain var compares on materialized values.
"""

from eudkit import *

# Damage curve for levels 0..31 (build-time data, monotonic).
DMG_TABLE = EUDArray(
    [
        10, 14, 19, 25, 32, 40, 49, 59,
        70, 82, 95, 109, 124, 140, 157, 175,
        194, 214, 235, 257, 280, 304, 329, 355,
        382, 410, 439, 469, 500, 532, 565, 599,
    ]
)
THRESHOLD = 300

level = EUDVariable(7)
damage = EUDVariable(0)
warn_lamp = EUDVariable(0)
crit_lamp = EUDVariable(0)
next_dmg = EUDVariable(0)


def run():
    global damage, warn_lamp, crit_lamp, next_dmg
    # Var-index lookup: damage for the current level.
    damage << DMG_TABLE[level]

    # Threshold test on the materialized value (var-vs-const).
    if EUDIf()(damage >= THRESHOLD):
        warn_lamp << 1
    if EUDElse()():
        warn_lamp << 0
    EUDEndIf()

    # Peek the NEXT level's damage and compare against 2x current
    # (var-vs-var compare — a basic legacy operation).
    next_dmg << DMG_TABLE[level + 1]
    if EUDIf()(next_dmg >= damage * 2):
        crit_lamp << 1
    if EUDElse()():
        crit_lamp << 0
    EUDEndIf()
