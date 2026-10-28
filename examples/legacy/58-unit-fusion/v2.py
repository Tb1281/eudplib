# 58 - unit fusion (modern eudkit)
#
# Expresses ideal intent:
# 1. Prioritized recipe evaluation (Recipe 2 Siege Tank > Recipe 1 Firebat).
# 2. Rising-edge material arrival detection for failed attempt accounting.
# 3. Clean local session variables and declarative action lists.
import eudkit


P1 = 0
MARINE = 0
FIREBAT = 10
SIEGE_TANK = 5
LOC_PAD = 4

R1_COST = 50
R2_COST = 100
FUSION_COOLDOWN = 30

from eudkit import session

cooldown = session.var(0)
fusions1 = session.var(0)
fusions2 = session.var(0)
failed_attempts = session.var(0)
materials_now = session.var(0)
prev_materials = session.var(0)

@session.fn(main_loop=True)
def game_loop() -> None:
    global cooldown, fusions1, fusions2, failed_attempts
    global materials_now, prev_materials

    if cooldown >= 1:
        cooldown -= 1

    has_firebats = 0
    has_marines = 0
    if eudkit.Bring(P1, eudkit.AtLeast, 2, FIREBAT, LOC_PAD):
        has_firebats = 1
    if eudkit.Bring(P1, eudkit.AtLeast, 2, MARINE, LOC_PAD):
        has_marines = 1

    materials_now = 0
    if has_firebats == 1:
        materials_now = 1
    elif has_marines == 1:
        materials_now = 1

    if cooldown == 0:
        if has_firebats == 1 and eudkit.Accumulate(P1, eudkit.AtLeast, R2_COST, eudkit.Ore):
            eudkit.RemoveUnitAt(2, FIREBAT, LOC_PAD, P1)
            eudkit.CreateUnit(1, SIEGE_TANK, LOC_PAD, P1)
            eudkit.SetResources(P1, eudkit.Subtract, R2_COST, eudkit.Ore)
            eudkit.MinimapPing(LOC_PAD)
            fusions2 += 1
            cooldown = FUSION_COOLDOWN
        elif has_marines == 1 and eudkit.Accumulate(P1, eudkit.AtLeast, R1_COST, eudkit.Ore):
            eudkit.RemoveUnitAt(2, MARINE, LOC_PAD, P1)
            eudkit.CreateUnit(1, FIREBAT, LOC_PAD, P1)
            eudkit.SetResources(P1, eudkit.Subtract, R1_COST, eudkit.Ore)
            eudkit.MinimapPing(LOC_PAD)
            fusions1 += 1
            cooldown = FUSION_COOLDOWN

        if materials_now == 1 and prev_materials == 0:
            if has_firebats == 1:
                if eudkit.Accumulate(P1, eudkit.AtMost, R2_COST - 1, eudkit.Ore):
                    failed_attempts += 1
            elif has_marines == 1:
                if eudkit.Accumulate(P1, eudkit.AtMost, R1_COST - 1, eudkit.Ore):
                    failed_attempts += 1

    prev_materials = materials_now
