"""Scenario 06: Arbitrary-Mask Bit Checking — Modern eudkit (v2).

Ideal Intent:
Check arbitrary bitmask flags in game state memory (`0x58A364 + 0x200`):
1. Test bit 0 (`0x00000001` - "Shield Ready") -> drive `lamp_shield` (1 or 0)
2. Test bit 5 (`0x00000020` - "Stealth Active") -> drive `lamp_stealth` (1 or 0)
3. Test bit 10 (`0x00000400` - "Hero Aura") -> drive `lamp_aura` (1 or 0)
4. Combined mask check: test if ANY of the three ability bits are active -> drive `lamp_any`.
"""

import eudkit
from eudkit import (
    session,
    EPD,
    dwread_epd,
)


FLAGS_EPD = EPD(0x58A364 + 0x200)

MASK_SHIELD = 0x00000001
MASK_STEALTH = 0x00000020
MASK_AURA = 0x00000400

lamp_shield = session.var(0, name="lamp_shield")
lamp_stealth = session.var(0, name="lamp_stealth")
lamp_aura = session.var(0, name="lamp_aura")
lamp_any = session.var(0, name="lamp_any")
flags = session.var(0, name="flags")

@session.fn
def game_loop():
    # Materialize flags once with optimized memory reader
    flags = dwread_epd(FLAGS_EPD)

    # Bitwise mask conditions using native & operator
    if flags & MASK_SHIELD != 0:
        lamp_shield = 1
    else:
        lamp_shield = 0

    if flags & MASK_STEALTH != 0:
        lamp_stealth = 1
    else:
        lamp_stealth = 0

    if flags & MASK_AURA != 0:
        lamp_aura = 1
    else:
        lamp_aura = 0

    # Composite mask testing
    if flags & (MASK_SHIELD | MASK_STEALTH | MASK_AURA) != 0:
        lamp_any = 1
    else:
        lamp_any = 0
