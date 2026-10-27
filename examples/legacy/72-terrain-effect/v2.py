# 72 - terrain effects: lava damage / spring healing (modern eudkit)
#
# Expresses ideal intent:
# 1. Periodic hazard processing using native `with session.every(12):` clock rule.
# 2. Direct unit list traversal with spatial bounds testing (lava vs spring).
# 3. Direct HP field reads and writebacks with zero global scratch variables.
from pathlib import Path
import eudkit

BASEMAP_PATH = Path(__file__).resolve().parents[3] / "tests" / "outputmap" / "basemap" / "basemap_strx.scx"

TICK_PERIOD = 12
LAVA_DAMAGE = 512
SPRING_HEAL = 256
HEAL_CAP = 10240
LAVA_X0, LAVA_X1, LAVA_Y0, LAVA_Y1 = 512, 1024, 2048, 2560
SPRING_X0, SPRING_X1, SPRING_Y0, SPRING_Y1 = 3072, 3584, 512, 1024

FIRST_VISIBLE_UNIT = 0x628430
DEATHS_BASE = 0x58A364


def build(session: eudkit.Session):
    lava_hits = session.var(0)
    lava_kills = session.var(0)
    heals = session.var(0)
    total_damage = session.var(0)

    uptr = session.var(0)
    uepd = session.var(0)
    scratch_hp = session.var(0)

    @session.fn(main_loop=True)
    def game_loop() -> None:
        nonlocal lava_hits, lava_kills, heals, total_damage, uptr, uepd, scratch_hp

        with session.every(TICK_PERIOD):
            uptr = eudkit.dwread(FIRST_VISIBLE_UNIT)
            while uptr != 0:
                uepd = (uptr - DEATHS_BASE) // 4
                x = eudkit.wread_epd(uepd + 10, 0)
                y = eudkit.wread_epd(uepd + 10, 1)

                if x >= LAVA_X0 and x <= LAVA_X1 and y >= LAVA_Y0 and y <= LAVA_Y1:
                    scratch_hp = eudkit.dwread_epd(uepd + 2)
                    if scratch_hp > LAVA_DAMAGE:
                        eudkit.dwwrite_epd(uepd + 2, scratch_hp - LAVA_DAMAGE)
                    else:
                        eudkit.dwwrite_epd(uepd + 2, 0)
                        lava_kills += 1
                    lava_hits += 1
                    total_damage += LAVA_DAMAGE

                if x >= SPRING_X0 and x <= SPRING_X1 and y >= SPRING_Y0 and y <= SPRING_Y1:
                    scratch_hp = eudkit.dwread_epd(uepd + 2)
                    if scratch_hp < HEAL_CAP:
                        eudkit.dwwrite_epd(uepd + 2, scratch_hp + SPRING_HEAL)
                        heals += 1

                uptr = eudkit.dwread_epd(uepd + 1)

    return {
        "lava_hits": lava_hits,
        "lava_kills": lava_kills,
        "heals": heals,
        "total_damage": total_damage,
    }


def main():
    session = eudkit.Session(BASEMAP_PATH)
    build(session)
    out_path = Path(__file__).parent / "output_v2.scx"
    report = session.compile(out_path)
    if out_path.exists():
        out_path.unlink()
    print("Scenario 72 (v2) compiled successfully:")
    print(f"  Triggers: {report.trigger_count}")
    print(f"  Payload:  {report.payload_size} bytes")
    print(f"  VProcs:   {report.vproc_count}")
    return report


if __name__ == "__main__":
    main()
