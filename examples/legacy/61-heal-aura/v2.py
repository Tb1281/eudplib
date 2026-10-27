# 61 - heal aura (modern eudkit)
#
# Expresses ideal intent:
# 1. Periodic pulse using native `with every(20):` clock rule.
# 2. Player unit list traversal (0x6283F8) with direct EPD offset addressing.
# 3. Exact HP deficit calculation and writeback without global scratch variables.
from pathlib import Path
import eudkit

BASEMAP_PATH = Path(__file__).resolve().parents[3] / "tests" / "outputmap" / "basemap" / "basemap_strx.scx"

P1 = 0
MARINE = 0

AURA_X = 1024
AURA_Y = 1024
AURA_RANGE = 256
X_MIN = AURA_X - AURA_RANGE
X_MAX = AURA_X + AURA_RANGE
Y_MIN = AURA_Y - AURA_RANGE
Y_MAX = AURA_Y + AURA_RANGE
MARINE_MAX_HP = 10240
HEAL_AMOUNT = 512
AURA_PERIOD = 20

FIRST_PLAYER_UNIT = 0x6283F8 + 4 * P1
DEATHS_BASE = 0x58A364


def build(session: eudkit.Session):
    scanned = session.var(0)
    total_scans = session.var(0)
    healed_last = session.var(0)
    heals_applied = session.var(0)
    hp_restored = session.var(0)

    uptr = session.var(0)
    uepd = session.var(0)

    @session.fn(main_loop=True)
    def game_loop() -> None:
        nonlocal scanned, total_scans, healed_last, heals_applied, hp_restored
        nonlocal uptr, uepd

        with session.every(AURA_PERIOD):
            scanned = 0
            healed_last = 0
            total_scans += 1

            uptr = eudkit.dwread(FIRST_PLAYER_UNIT)
            while uptr != 0:
                uepd = (uptr - DEATHS_BASE) // 4
                scanned += 1
                unit_type = eudkit.wread_epd(uepd + 25, 0)
                if unit_type == MARINE:
                    x = eudkit.wread_epd(uepd + 10, 0)
                    if x >= X_MIN and x <= X_MAX:
                        y = eudkit.wread_epd(uepd + 10, 1)
                        if y >= Y_MIN and y <= Y_MAX:
                            hp = eudkit.dwread_epd(uepd + 2)
                            if hp < MARINE_MAX_HP:
                                if hp <= MARINE_MAX_HP - HEAL_AMOUNT:
                                    hp += HEAL_AMOUNT
                                    hp_restored += HEAL_AMOUNT
                                    eudkit.dwwrite_epd(uepd + 2, hp)
                                else:
                                    deficit = MARINE_MAX_HP - hp
                                    hp_restored += deficit
                                    eudkit.dwwrite_epd(uepd + 2, MARINE_MAX_HP)
                                healed_last += 1
                                heals_applied += 1
                uptr = eudkit.dwread_epd(uepd + 27)

    return {
        "scanned": scanned,
        "total_scans": total_scans,
        "healed_last": healed_last,
        "heals_applied": heals_applied,
        "hp_restored": hp_restored,
    }


def main():
    session = eudkit.Session(BASEMAP_PATH)
    build(session)
    out_path = Path(__file__).parent / "output_v2.scx"
    report = session.compile(out_path)
    if out_path.exists():
        out_path.unlink()
    print("Scenario 61 (v2) compiled successfully:")
    print(f"  Triggers: {report.trigger_count}")
    print(f"  Payload:  {report.payload_size} bytes")
    print(f"  VProcs:   {report.vproc_count}")
    return report


if __name__ == "__main__":
    main()
