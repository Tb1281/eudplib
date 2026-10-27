# 64 - defeat / elimination handling (modern eudkit)
#
# Expresses ideal intent:
# 1. First-appearance detection for active players via Command Center checks.
# 2. Complete board cleanup via stock `eudkit.KillUnit(ANY_UNIT, p)` action.
# 3. Last-man-standing victory detection and string announcements.
from pathlib import Path
import eudkit

BASEMAP_PATH = Path(__file__).resolve().parents[3] / "tests" / "outputmap" / "basemap" / "basemap_strx.scx"

COMMAND_CENTER = 106
ANY_UNIT = 229


def build(session: eudkit.Session):
    started = [session.var(0) for _ in range(4)]
    out = [session.var(0) for _ in range(4)]
    alive_count = session.var(0)
    winner = session.var(0)
    game_over = session.var(0)

    msg_eliminated = session.map_string("A command center has fallen - a player is eliminated.")
    msg_victory = session.map_string("One player remains - victory!")

    @session.fn(main_loop=True)
    def game_loop() -> None:
        nonlocal alive_count, winner, game_over

        for p in range(4):
            if started[p] == 0 and eudkit.Command(p, eudkit.AtLeast, 1, COMMAND_CENTER):
                started[p] = 1
                alive_count += 1

            if started[p] == 1 and out[p] == 0 and eudkit.Command(p, eudkit.Exactly, 0, COMMAND_CENTER):
                out[p] = 1
                alive_count -= 1
                eudkit.KillUnit(ANY_UNIT, p)
                msg_eliminated.display()

        if game_over == 0 and alive_count == 1:
            for p in range(4):
                if started[p] == 1 and out[p] == 0:
                    winner = p + 1
            msg_victory.display()
            game_over = 1

    return {
        "alive_count": alive_count,
        "winner": winner,
        "game_over": game_over,
    }


def main():
    session = eudkit.Session(BASEMAP_PATH)
    build(session)
    out_path = Path(__file__).parent / "output_v2.scx"
    report = session.compile(out_path)
    if out_path.exists():
        out_path.unlink()
    print("Scenario 64 (v2) compiled successfully:")
    print(f"  Triggers: {report.trigger_count}")
    print(f"  Payload:  {report.payload_size} bytes")
    print(f"  VProcs:   {report.vproc_count}")
    return report


if __name__ == "__main__":
    main()
