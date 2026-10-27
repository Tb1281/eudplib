# 38 - elapsed-time staged events (modern eudkit)
#
# Expresses ideal intent:
# 1. Native `ElapsedTime` condition cleanly scheduled with declarative `with once():` blocks.
# 2. Eliminates the legacy `stage` variable and its redundant per-tick state checks.
# 3. Clean local session state with zero VProcs.
from pathlib import Path
import eudkit

BASEMAP_PATH = Path(__file__).resolve().parents[3] / "tests" / "outputmap" / "basemap" / "basemap_strx.scx"

MARINE = 0
ZERGLING = 37
HYDRALISK = 38
ULTRALISK = 39

SPAWN = 1
SPAWN_B = 2


def build(session: eudkit.Session):
    @session.fn(main_loop=True)
    def game_loop() -> None:
        if eudkit.ElapsedTime(eudkit.AtLeast, 10):
            with eudkit.once():
                eudkit.DisplayText("Get ready - first wave in 20 seconds.")

        if eudkit.ElapsedTime(eudkit.AtLeast, 30):
            with eudkit.once():
                eudkit.DisplayText("Wave 1 incoming!")
                eudkit.CreateUnit(4, MARINE, SPAWN, eudkit.P2)

        if eudkit.ElapsedTime(eudkit.AtLeast, 90):
            with eudkit.once():
                eudkit.DisplayText("Wave 2 incoming!")
                eudkit.CreateUnit(4, ZERGLING, SPAWN, eudkit.P2)
                eudkit.CreateUnit(4, ZERGLING, SPAWN_B, eudkit.P2)

        if eudkit.ElapsedTime(eudkit.AtLeast, 180):
            with eudkit.once():
                eudkit.DisplayText("WARNING: something huge approaches...")
                eudkit.CreateUnit(1, ULTRALISK, SPAWN, eudkit.P2)
                eudkit.CreateUnit(4, HYDRALISK, SPAWN_B, eudkit.P2)

        if eudkit.ElapsedTime(eudkit.AtLeast, 240):
            with eudkit.once():
                eudkit.DisplayText("Final wave! Hold the line!")
                eudkit.CreateUnit(6, HYDRALISK, SPAWN, eudkit.P2)
                eudkit.CreateUnit(6, HYDRALISK, SPAWN_B, eudkit.P2)


def main():
    session = eudkit.Session(BASEMAP_PATH)
    build(session)
    out_path = Path(__file__).parent / "output_v2.scx"
    report = session.compile(out_path)
    if out_path.exists():
        out_path.unlink()
    print("Scenario 38 (v2) compiled successfully:")
    print(f"  Triggers: {report.trigger_count}")
    print(f"  Payload:  {report.payload_size} bytes")
    print(f"  VProcs:   {report.vproc_count}")
    return report


if __name__ == "__main__":
    main()
