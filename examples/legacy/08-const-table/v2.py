"""Scenario 08: Constant Lookup Table (Damage-by-Level) — Modern eudkit (v2).

Ideal Intent:
Lookup stats from a 32-entry build-time damage progression table:
1. Lookup current level's damage: `damage = dmg_table[level]`
2. Threshold comparison: activate warning lamp if `damage >= 300`
3. Lookup next level's damage: `next_dmg = dmg_table[level + 1]`
4. Dynamic var-vs-var comparison: activate critical lamp if `next_dmg >= damage * 2`.
"""

import eudkit
from eudkit import (
    session,
    Array,
    EPD,
)


DMG_TABLE_DATA = [
    10, 14, 19, 25, 32, 40, 49, 59,
    70, 82, 95, 109, 124, 140, 157, 175,
    194, 214, 235, 257, 280, 304, 329, 355,
    382, 410, 439, 469, 500, 532, 565, 599,
]
DMG_TABLE_EPD = EPD(0x6FE000)
THRESHOLD = 300

dmg_table = Array(DMG_TABLE_DATA, epd=DMG_TABLE_EPD, session=session)
level = session.var(7, name="level")
damage = session.var(0, name="damage")
warn_lamp = session.var(0, name="warn_lamp")
crit_lamp = session.var(0, name="crit_lamp")
next_dmg = session.var(0, name="next_dmg")

@session.fn
def game_loop():
    # Dynamic variable-indexed table lookup
    damage = dmg_table[level]

    # Constant threshold comparison
    if damage >= THRESHOLD:
        warn_lamp = 1
    else:
        warn_lamp = 0

    # Peeking next level with arithmetic index expression (level + 1)
    next_dmg = dmg_table[level + 1]

    # Dynamic variable-versus-variable comparison with scaled operand
    if next_dmg >= damage * 2:
        crit_lamp = 1
    else:
        crit_lamp = 0
