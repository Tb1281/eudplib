# 34 - units.dat patch (legacy eudplib)
#
# Intent: at game start, buff every Terran infantry unit type by writing
# its Units.dat row directly (max HP and armor), then read the values
# back and announce success only if every read-back matches.
#
# Counting form (pilot convention): run() builds the scenario's
# triggers once; benchmarks/pilot/count_legacy.py calls it inside a
# fresh PushTriggerScope and reports the GetTriggerCounter() delta.
# In production this body is the onPluginStart hook. Units and text
# are referenced by id / Db string (no basemap in the harness, exactly
# like the pilot scenarios); a real map build resolves the same calls
# by name/literal and creates the identical triggers. Module-level
# declarations and the import-time shared engines (readtable, 80T,
# session-amortized) are outside the delta.
from eudkit import *

# Units.dat in-memory field bases (DefDataDict / classic EUD addresses).
HP_BASE = 0x662350      # hitPoints: dword per unit id, stored as HP * 256
ARMOR_BASE = 0x65FEC8   # armor: byte per unit id

MARINE, GHOST, FIREBAT, MEDIC = 0, 1, 32, 34

verify_failed = EUDVariable()


@EUDFunc
def patch_unit(unit_id, new_hp):
    # hitPoints are stored shifted: real HP = stored value / 256.
    f_dwwrite(HP_BASE + 4 * unit_id, new_hp * 256)
    # armor is a plain byte.
    f_bwrite(ARMOR_BASE + unit_id, 1)


@EUDFunc
def verify_unit(unit_id, expected_hp):
    global verify_failed
    hp = f_dwread(HP_BASE + 4 * unit_id)
    armor = f_bread(ARMOR_BASE + unit_id)
    if EUDIf()(hp != expected_hp * 256):
        verify_failed += 1
    EUDEndIf()
    if EUDIf()(armor != 1):
        verify_failed += 1
    EUDEndIf()


def run():
    verify_failed << 0

    patch_unit(MARINE, 80)
    patch_unit(GHOST, 60)
    patch_unit(FIREBAT, 70)
    patch_unit(MEDIC, 75)

    verify_unit(MARINE, 80)
    verify_unit(GHOST, 60)
    verify_unit(FIREBAT, 70)
    verify_unit(MEDIC, 75)

    if EUDIf()(verify_failed == 0):
        DoActions(DisplayText(Db("Terran infantry buffed: HP up, armor 1.")))
    EUDEndIf()
    if EUDIf()(verify_failed >= 1):
        DoActions(DisplayText(Db("WARNING: infantry buff did not stick.")))
    EUDEndIf()
