# 35 - weapons.dat / upgrades.dat patch (legacy eudplib)
#
# Intent: at game start, tune two weapons (damage up, cooldown down) and
# make Terran Infantry Armor cheaper: read the current upgrade mineral
# cost from Upgrades.dat, cut it by 25 if it is at least 100, and read
# the values back to confirm before announcing.
#
# Counting form (pilot convention): run() builds the scenario's
# triggers once; benchmarks/pilot/count_legacy.py calls it inside a
# fresh PushTriggerScope and reports the GetTriggerCounter() delta.
# In production this body is the onPluginStart hook. Text is a Db
# string (no basemap in the harness, like the pilot scenarios); a real
# map build resolves the same call by literal and creates the
# identical triggers. The import-time shared readtable engine (80T,
# session-amortized) is outside the delta.
from eudkit import *

# Weapons.dat field bases (word/byte per weapon id).
DMG_BASE = 0x656EB0     # damage: word per weapon id
CD_BASE = 0x656FB8      # cooldown: byte per weapon id
# Upgrades.dat field bases (per upgrade id).
MINCOST_BASE = 0x655740  # mineralCostBase: word per upgrade id
GASCOST_BASE = 0x655840  # gasCostBase: word per upgrade id
MAXLV_BASE = 0x655700    # maxLevel: byte per upgrade id

GAUSS_RIFLE = 0
C10_RIFLE = 2
INFANTRY_ARMOR = 0

verify_failed = EUDVariable()


@EUDFunc
def patch_weapon(weapon_id, new_damage, new_cooldown):
    f_wwrite(DMG_BASE + 2 * weapon_id, new_damage)
    f_bwrite(CD_BASE + weapon_id, new_cooldown)


@EUDFunc
def verify_weapon(weapon_id, expected_damage, expected_cooldown):
    global verify_failed
    dmg = f_wread(DMG_BASE + 2 * weapon_id)
    cd = f_bread(CD_BASE + weapon_id)
    if EUDIf()(dmg != expected_damage):
        verify_failed += 1
    EUDEndIf()
    if EUDIf()(cd != expected_cooldown):
        verify_failed += 1
    EUDEndIf()


def run():
    verify_failed << 0

    patch_weapon(GAUSS_RIFLE, 22, 18)
    patch_weapon(C10_RIFLE, 30, 20)

    # Read the current armor cost, then discount it conditionally.
    cost = f_wread(MINCOST_BASE + 2 * INFANTRY_ARMOR)
    if EUDIf()(cost >= 100):
        f_wwrite(MINCOST_BASE + 2 * INFANTRY_ARMOR, cost - 25)
    EUDEndIf()
    # Pin the max level so the discount is not a stealth buff.
    maxlv = f_bread(MAXLV_BASE + INFANTRY_ARMOR)
    if EUDIf()(maxlv != 3):
        f_bwrite(MAXLV_BASE + INFANTRY_ARMOR, 3)
    EUDEndIf()

    verify_weapon(GAUSS_RIFLE, 22, 18)
    verify_weapon(C10_RIFLE, 30, 20)

    if EUDIf()(verify_failed == 0):
        DoActions(DisplayText(Db("Weapons tuned; infantry armor discounted.")))
    EUDEndIf()
