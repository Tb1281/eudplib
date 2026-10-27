# 58 - unit fusion (legacy eudplib)
#
# Intent: a fusion pad (location 4) where Player 1 combines units
# into stronger ones, Templar-Archives style:
#   Recipe 2 (checked first): 2 Firebats + 100 ore -> 1 Siege Tank
#   Recipe 1:                 2 Marines  +  50 ore -> 1 Firebat
# Materials are consumed with RemoveUnitAt, the product is created
# on the pad, the ore is charged, and a shared 30-loop cooldown
# prevents chain fusions. Recipe 2 has priority: when both are
# possible, the tank is built. Blocked attempts are detected on the
# RISING EDGE of "materials present": if the materials for a recipe
# are on the pad, the cooldown is idle, but the ore is short,
# exactly one failed attempt is counted per arrival.
#
# Counting form (pilot convention): run() builds the scenario's
# triggers once; benchmarks/pilot/count_legacy.py calls it inside a
# fresh PushTriggerScope and reports the GetTriggerCounter() delta.
# In production the body is the beforeTriggerExec hook; run() is
# that body verbatim. Units and locations are numeric ids (no
# basemap in the harness, exactly like the pilot scenarios).
from eudkit import *

P1 = 0
MARINE = 0          # unit ids
FIREBAT = 10
SIEGE_TANK = 5
LOC_PAD = 4         # location id

R1_COST = 50        # ore for recipe 1 (2 Marines -> Firebat)
R2_COST = 100       # ore for recipe 2 (2 Firebats -> Tank)
FUSION_COOLDOWN = 30  # loops between fusions

cooldown = EUDVariable(0)       # loops until the pad is ready
fusions1 = EUDVariable(0)       # lifetime recipe-1 fusions
fusions2 = EUDVariable(0)       # lifetime recipe-2 fusions
failed_attempts = EUDVariable(0)  # material-present, ore-short
materials_now = EUDVariable(0)  # any recipe's materials present
prev_materials = EUDVariable(0)  # materials_now at last loop
has_firebats = EUDVariable(0)   # scratch flags (legacy style:
has_marines = EUDVariable(0)    # compound Or is a flag, not a Cond)


def run():
    global cooldown, fusions1, fusions2, failed_attempts
    global materials_now, prev_materials, has_firebats, has_marines
    if EUDIf()(cooldown >= 1):
        cooldown -= 1
    EUDEndIf()

    # Materials presence (either recipe), for the edge detector.
    has_firebats << 0
    has_marines << 0
    if EUDIf()(Bring(P1, AtLeast, 2, FIREBAT, LOC_PAD)):
        has_firebats << 1
    EUDEndIf()
    if EUDIf()(Bring(P1, AtLeast, 2, MARINE, LOC_PAD)):
        has_marines << 1
    EUDEndIf()
    materials_now << 0
    if EUDIf()(has_firebats == 1):
        materials_now << 1
    EUDEndIf()
    if EUDIf()(has_marines == 1):
        materials_now << 1
    EUDEndIf()

    if EUDIf()(cooldown == 0):
        # Recipe 2 first: the tank outranks the firebat.
        if EUDIf()(has_firebats == 1):
            if EUDIf()(Accumulate(P1, AtLeast, R2_COST, Ore)):
                DoActions([
                    RemoveUnitAt(2, FIREBAT, LOC_PAD, P1),
                    CreateUnit(1, SIEGE_TANK, LOC_PAD, P1),
                    SetResources(P1, Subtract, R2_COST, Ore),
                    MinimapPing(LOC_PAD),
                ])
                fusions2 += 1
                cooldown << FUSION_COOLDOWN
            EUDEndIf()
        EUDEndIf()

        # Recipe 1: only if the pad is still idle (recipe 2 may
        # have just consumed the cooldown this same loop).
        if EUDIf()(cooldown == 0):
            if EUDIf()(has_marines == 1):
                if EUDIf()(Accumulate(P1, AtLeast, R1_COST, Ore)):
                    DoActions([
                        RemoveUnitAt(2, MARINE, LOC_PAD, P1),
                        CreateUnit(1, FIREBAT, LOC_PAD, P1),
                        SetResources(P1, Subtract, R1_COST, Ore),
                        MinimapPing(LOC_PAD),
                    ])
                    fusions1 += 1
                    cooldown << FUSION_COOLDOWN
                EUDEndIf()
            EUDEndIf()
        EUDEndIf()

        # Blocked attempt: materials arrived (rising edge) but the
        # ore for the available recipe is short.
        if EUDIf()(materials_now == 1):
            if EUDIf()(prev_materials == 0):
                if EUDIf()(has_firebats == 1):
                    if EUDIf()(Accumulate(P1, AtMost, R2_COST - 1, Ore)):
                        failed_attempts += 1
                    EUDEndIf()
                EUDEndIf()
                if EUDIf()(has_firebats == 0):
                    if EUDIf()(has_marines == 1):
                        if EUDIf()(Accumulate(P1, AtMost, R1_COST - 1, Ore)):
                            failed_attempts += 1
                        EUDEndIf()
                    EUDEndIf()
                EUDEndIf()
            EUDEndIf()
        EUDEndIf()
    EUDEndIf()

    prev_materials << materials_now
