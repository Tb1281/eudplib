# 58 - unit fusion (modern eudkit)
#
# Expresses ideal intent:
# 1. Prioritized recipe evaluation (Recipe 2 Siege Tank > Recipe 1 Firebat).
# 2. Rising-edge material arrival detection for failed attempt accounting.
# 3. Clean local session variables and declarative action lists.
from pathlib import Path
import eudkit

BASEMAP_PATH = Path(__file__).resolve().parents[3] / "tests" / "outputmap" / "basemap" / "basemap_strx.scx"

P1 = 0
MARINE = 0
FIREBAT = 10
SIEGE_TANK = 5
LOC_PAD = 4

R1_COST = 50
R2_COST = 100
FUSION_COOLDOWN = 30


def build(session: eudkit.Session):
    cooldown = session.var(0)
    fusions1 = session.var(0)
    fusions2 = session.var(0)
    failed_attempts = session.var(0)
    materials_now = session.var(0)
    prev_materials = session.var(0)

    @session.fn(main_loop=True)
    def game_loop() -> None:
        nonlocal cooldown, fusions1, fusions2, failed_attempts
        nonlocal materials_now, prev_materials

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

    return {
        "cooldown": cooldown,
        "fusions1": fusions1,
        "fusions2": fusions2,
        "failed_attempts": failed_attempts,
    }


def main():
    session = eudkit.Session(BASEMAP_PATH)
    build(session)
    out_path = Path(__file__).parent / "output_v2.scx"
    report = session.compile(out_path)
    if out_path.exists():
        out_path.unlink()
    print("Scenario 58 (v2) compiled successfully:")
    print(f"  Triggers: {report.trigger_count}")
    print(f"  Payload:  {report.payload_size} bytes")
    print(f"  VProcs:   {report.vproc_count}")
    return report


if __name__ == "__main__":
    main()
