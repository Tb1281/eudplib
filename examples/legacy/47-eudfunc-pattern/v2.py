# 47 - function-call pattern: inlined functions (modern eudkit)
#
# Expresses ideal intent:
# 1. Inlined function helpers via `@session.fn(inline=True)` eliminating
#    function frame setups, callee jump triggers, and return trampolines.
# 2. Direct mathematical expression lowering for clamping and squared distance.
# 3. Clean local session state with minimal VProcs.
from pathlib import Path
import eudkit

BASEMAP_PATH = Path(__file__).resolve().parents[3] / "tests" / "outputmap" / "basemap" / "basemap_strx.scx"


def build(session: eudkit.Session):
    @session.fn(inline=True)
    def calc_damage(atk, defense):
        dmg = session.var(1)
        if atk > defense:
            dmg = atk - defense
        return dmg

    @session.fn(inline=True)
    def dist2(x1, y1, x2, y2):
        dx = x1 - x2
        dy = y1 - y2
        res = session.var(0)
        res = dx * dx + dy * dy
        return res

    hero_atk = session.var(50)
    hero_def = session.var(20)
    last_damage = session.var(0)
    last_dist2 = session.var(0)
    critical = session.var(0)
    hit_count = session.var(0)
    bonus_damage = session.var(0)

    @session.fn(main_loop=True)
    def game_loop() -> None:
        nonlocal hero_atk, hero_def, last_damage, last_dist2
        nonlocal critical, hit_count, bonus_damage

        hero_atk = 50
        hero_def = 20

        last_damage = calc_damage(hero_atk, hero_def)
        last_dist2 = dist2(100, 100, 160, 130)

        if last_damage > 30:
            critical = 1
            hit_count += 1
        else:
            critical = 0

        bonus_damage = calc_damage(80, 15)
        if bonus_damage >= 60:
            hero_atk += 5

        if last_dist2 <= 10000:
            eudkit.DisplayText("In range")

    return {
        "hero_atk": hero_atk,
        "last_damage": last_damage,
        "last_dist2": last_dist2,
        "hit_count": hit_count,
    }


def main():
    session = eudkit.Session(BASEMAP_PATH)
    build(session)
    out_path = Path(__file__).parent / "output_v2.scx"
    report = session.compile(out_path)
    if out_path.exists():
        out_path.unlink()
    print("Scenario 47 (v2) compiled successfully:")
    print(f"  Triggers: {report.trigger_count}")
    print(f"  Payload:  {report.payload_size} bytes")
    print(f"  VProcs:   {report.vproc_count}")
    return report


if __name__ == "__main__":
    main()
