# 59 - hero revive (modern eudkit)
#
# Expresses ideal intent:
# 1. State machine for hero death detection and progressive revive delays.
# 2. Map strings for zero-overhead static announcements.
# 3. Clean local session variables and declarative action lists.
from pathlib import Path
import eudkit

BASEMAP_PATH = Path(__file__).resolve().parents[3] / "tests" / "outputmap" / "basemap" / "basemap_strx.scx"

P1 = 0
MARINE = 0
LOC_BASE = 1

BASE_WAIT = 180
WAIT_PER_DEATH = 30
REVIVE_COST = 100


def build(session: eudkit.Session):
    hero_state = session.var(0)     # 0 = alive, 1 = dead
    countdown = session.var(0)
    waiting = session.var(0)
    deaths = session.var(0)
    revives = session.var(0)
    total_dead_loops = session.var(0)
    total_fees = session.var(0)

    msg_fallen = session.map_string("Hero has fallen")
    msg_returned = session.map_string("Hero has returned")

    @session.fn(main_loop=True)
    def game_loop() -> None:
        nonlocal hero_state, countdown, waiting, deaths, revives
        nonlocal total_dead_loops, total_fees

        if hero_state == 0:
            if eudkit.Command(P1, eudkit.Exactly, 0, MARINE):
                hero_state = 1
                deaths += 1
                countdown = BASE_WAIT + deaths * WAIT_PER_DEATH
                eudkit.MinimapPing(LOC_BASE)
                msg_fallen.display()

        if hero_state == 1:
            total_dead_loops += 1
            if countdown >= 1:
                countdown -= 1

            if countdown == 0:
                if eudkit.Accumulate(P1, eudkit.AtLeast, REVIVE_COST, eudkit.Ore):
                    eudkit.SetResources(P1, eudkit.Subtract, REVIVE_COST, eudkit.Ore)
                    eudkit.CreateUnit(1, MARINE, LOC_BASE, P1)
                    eudkit.MinimapPing(LOC_BASE)
                    msg_returned.display()
                    total_fees += REVIVE_COST
                    hero_state = 0
                    waiting = 0
                    revives += 1
                else:
                    waiting = 1

    return {
        "hero_state": hero_state,
        "countdown": countdown,
        "waiting": waiting,
        "deaths": deaths,
        "revives": revives,
        "total_dead_loops": total_dead_loops,
        "total_fees": total_fees,
    }


def main():
    session = eudkit.Session(BASEMAP_PATH)
    build(session)
    out_path = Path(__file__).parent / "output_v2.scx"
    report = session.compile(out_path)
    if out_path.exists():
        out_path.unlink()
    print("Scenario 59 (v2) compiled successfully:")
    print(f"  Triggers: {report.trigger_count}")
    print(f"  Payload:  {report.payload_size} bytes")
    print(f"  VProcs:   {report.vproc_count}")
    return report


if __name__ == "__main__":
    main()
