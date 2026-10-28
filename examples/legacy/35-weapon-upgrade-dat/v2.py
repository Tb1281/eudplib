"""Scenario 35: Weapons.dat and Upgrades.dat In-Memory Patch — Modern eudkit (v2).

Ideal Intent:
At map initialization, patch weapons and upgrade costs in memory:
1. Gauss Rifle (weapon 0): 22 damage (word), 18 cooldown (byte).
2. C-10 Rifle (weapon 2): 30 damage (word), 20 cooldown (byte).
3. Terran Infantry Armor (upgrade 0):
   - Reads current mineral base cost; if >= 100, discounts by 25.
   - Pins max upgrade level to 3.
4. Reads back weapon values to verify patches:
   - If all match: announces `"Weapons tuned; infantry armor discounted."`
Modern v2 uses:
- Declarative `with once():` lifecycle block.
- Direct typed memory word/byte primitives (`wwrite`, `bwrite`, `wread`, `bread`).
- STRx-backed literal strings in `DisplayText`.
"""

import eudkit
from eudkit import (
    session,
    DisplayText,
    bread,
    bwrite,
    once,
    wread,
    wwrite,
)


# Weapons.dat field bases
DMG_BASE = 0x656EB0      # word per weapon id
CD_BASE = 0x656FB8       # byte per weapon id
# Upgrades.dat field bases
MINCOST_BASE = 0x655740  # word per upgrade id
MAXLV_BASE = 0x655700    # byte per upgrade id

GAUSS_RIFLE = 0
C10_RIFLE = 2
INFANTRY_ARMOR = 0

verify_failed = session.var(0, name="verify_failed")

@session.fn
def init_patch():
    with once():
        verify_failed = 0

        # 1. Patch weapons
        wwrite(DMG_BASE + 2 * GAUSS_RIFLE, 22)
        bwrite(CD_BASE + GAUSS_RIFLE, 18)

        wwrite(DMG_BASE + 2 * C10_RIFLE, 30)
        bwrite(CD_BASE + C10_RIFLE, 20)

        # 2. Read armor cost and discount conditionally
        cost = wread(MINCOST_BASE + 2 * INFANTRY_ARMOR)
        if cost >= 100:
            wwrite(MINCOST_BASE + 2 * INFANTRY_ARMOR, cost - 25)

        maxlv = bread(MAXLV_BASE + INFANTRY_ARMOR)
        if maxlv != 3:
            bwrite(MAXLV_BASE + INFANTRY_ARMOR, 3)

        # 3. Read back and verify weapon patches
        dmg0 = wread(DMG_BASE + 2 * GAUSS_RIFLE)
        cd0 = bread(CD_BASE + GAUSS_RIFLE)
        if dmg0 != 22:
            verify_failed += 1
        if cd0 != 18:
            verify_failed += 1

        dmg2 = wread(DMG_BASE + 2 * C10_RIFLE)
        cd2 = bread(CD_BASE + C10_RIFLE)
        if dmg2 != 30:
            verify_failed += 1
        if cd2 != 20:
            verify_failed += 1

        # 4. Announce success
        if verify_failed == 0:
            DisplayText("Weapons tuned; infantry armor discounted.")
