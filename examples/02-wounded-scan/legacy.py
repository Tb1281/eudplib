"""LEGACY wounded-unit scan (pilot 2).

Same intent as ours_2_wounded_scan.js: walk the live unit list every
trigger loop and census the wounded. For each unit the HP field
(CUnit + 0x08, a dword in 1/256 HP units) is read with
f_dwread_epd into a variable and compared against two thresholds:
wounded (< 50% of a Marine's 40 HP) and critical (< 25%). Tallies are
kept per loop and latched into report variables for display code
elsewhere.
"""

from eudkit import *

# HP thresholds in the engine's fixed-point units (HP * 256).
# A Marine has 40 HP -> 10240. Wounded < 5120, critical < 2560.
WOUNDED_MAX = 5120
CRITICAL_MAX = 2560
HP_EPD_OFFSET = 2  # CUnit + 0x08 byte offset, in EPD (dword) units

# Census state.
hp = EUDVariable()               # current unit's HP (read per unit)
scanned = EUDVariable()          # units seen this loop
wounded = EUDVariable()          # units below WOUNDED_MAX
critical = EUDVariable()         # units below CRITICAL_MAX
report_scanned = EUDVariable()   # latched end-of-loop copies
report_wounded = EUDVariable()
report_critical = EUDVariable()
alarm = EUDVariable()            # 1 while any critical unit exists


def run():
    global hp, scanned, wounded, critical
    global report_scanned, report_wounded, report_critical, alarm
    # Reset the per-loop tallies.
    scanned << 0
    wounded << 0
    critical << 0

    # Live-list traversal; the loop yields (ptr, epd) per unit.
    for uptr, uepd in EUDLoopUnit():
        scanned += 1
        # Read the HP dword via the unit's EPD (+2 dwords = +0x08).
        hp << f_dwread_epd(uepd + HP_EPD_OFFSET)
        # Dead-check: a unit mid-removal can read 0 HP; skip it.
        if EUDIf()(hp == 0):
            EUDContinue()
        EUDEndIf()
        if EUDIf()(hp < WOUNDED_MAX):
            wounded += 1
            if EUDIf()(hp < CRITICAL_MAX):
                critical += 1
            EUDEndIf()
        EUDEndIf()

    # Latch this loop's census for the display code and raise the
    # alarm while at least one critical unit is on the field.
    report_scanned << scanned
    report_wounded << wounded
    report_critical << critical
    if EUDIf()(critical >= 1):
        alarm << 1
    EUDEndIf()
    if EUDIf()(critical == 0):
        alarm << 0
    EUDEndIf()
