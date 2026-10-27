# 70 - enemy AI director (modern eudkit)
#
# Expresses ideal intent:
# 1. Periodic AI decisions using native `with session.every(90):` clock rule.
# 2. Army threshold evaluation and order issuance using stock `Order` actions (Attack, Patrol, Move).
# 3. Automatic reinforcement production with unit cap checks.
from pathlib import Path
import eudkit

BASEMAP_PATH = Path(__file__).resolve().parents[3] / "tests" / "outputmap" / "basemap" / "basemap_strx.scx"

MARINE = 0
ENEMY = 7
ENEMY_BASE = 50
MUSTER_FIELD = 52
PLAYER_BASE = 51
MIDFIELD = 54
RALLY = 53
DECISION_PERIOD = 90
ATTACK_THRESHOLD = 12
RETREAT_THRESHOLD = 3
REINFORCE_CAP = 20
REINFORCE_BATCH = 2


def build(session: eudkit.Session):
    attack_waves = session.var(0)
    patrol_orders = session.var(0)
    retreats = session.var(0)
    reinforcements = session.var(0)
    last_stance = session.var(0)

    @session.fn(main_loop=True)
    def game_loop() -> None:
        nonlocal attack_waves, patrol_orders, retreats, reinforcements, last_stance

        with session.every(DECISION_PERIOD):
            if eudkit.Command(ENEMY, eudkit.AtLeast, ATTACK_THRESHOLD, MARINE):
                eudkit.Order(MARINE, ENEMY, MUSTER_FIELD, eudkit.Attack, PLAYER_BASE)
                attack_waves += 1
                last_stance = 1
            elif eudkit.Command(ENEMY, eudkit.AtLeast, RETREAT_THRESHOLD + 1, MARINE):
                eudkit.Order(MARINE, ENEMY, MUSTER_FIELD, eudkit.Patrol, MIDFIELD)
                patrol_orders += 1
                last_stance = 2
            else:
                eudkit.Order(MARINE, ENEMY, MUSTER_FIELD, eudkit.Move, RALLY)
                retreats += 1
                last_stance = 3

            if eudkit.Command(ENEMY, eudkit.AtMost, REINFORCE_CAP - 1, MARINE):
                eudkit.CreateUnit(REINFORCE_BATCH, MARINE, ENEMY_BASE, ENEMY)
                reinforcements += REINFORCE_BATCH

    return {
        "attack_waves": attack_waves,
        "patrol_orders": patrol_orders,
        "retreats": retreats,
        "reinforcements": reinforcements,
        "last_stance": last_stance,
    }


def main():
    session = eudkit.Session(BASEMAP_PATH)
    build(session)
    out_path = Path(__file__).parent / "output_v2.scx"
    report = session.compile(out_path)
    if out_path.exists():
        out_path.unlink()
    print("Scenario 70 (v2) compiled successfully:")
    print(f"  Triggers: {report.trigger_count}")
    print(f"  Payload:  {report.payload_size} bytes")
    print(f"  VProcs:   {report.vproc_count}")
    return report


if __name__ == "__main__":
    main()
