# 41 - minimap ping + warning with cooldown (modern eudkit)
#
# Expresses ideal intent:
# 1. Native `Bring` condition and `MinimapPing` stock trigger action.
# 2. Local countdown timers managed without global singletons.
# 3. Clean local session state with zero VProcs.
from pathlib import Path
import eudkit

BASEMAP_PATH = Path(__file__).resolve().parents[3] / "tests" / "outputmap" / "basemap" / "basemap_strx.scx"

COOLDOWN_FRAMES = 168   # ~10 s at Fastest (16.7 trigger loops / s)
ENEMY = eudkit.P2

MARINE = 0
HYDRALISK = 38
ZERGLING = 37

WATCH_ZONE = 1
GATE_ZONE = 2
REAR_ZONE = 3


def build(session: eudkit.Session):
    watch_cd = session.var(0)
    gate_cd = session.var(0)
    rear_cd = session.var(0)

    @session.fn(main_loop=True)
    def game_loop() -> None:
        nonlocal watch_cd, gate_cd, rear_cd
        if watch_cd >= 1:
            watch_cd -= 1
        if gate_cd >= 1:
            gate_cd -= 1
        if rear_cd >= 1:
            rear_cd -= 1

        if eudkit.Bring(ENEMY, eudkit.AtLeast, 1, MARINE, WATCH_ZONE):
            if watch_cd == 0:
                watch_cd = COOLDOWN_FRAMES
                eudkit.MinimapPing(WATCH_ZONE)
                eudkit.DisplayText("Warning: enemy Marine in the Watch Zone!")

        if eudkit.Bring(ENEMY, eudkit.AtLeast, 1, HYDRALISK, GATE_ZONE):
            if gate_cd == 0:
                gate_cd = COOLDOWN_FRAMES
                eudkit.MinimapPing(GATE_ZONE)
                eudkit.DisplayText("Warning: enemy Hydralisk at the Gate Zone!")

        if eudkit.Bring(ENEMY, eudkit.AtLeast, 1, ZERGLING, REAR_ZONE):
            if rear_cd == 0:
                rear_cd = COOLDOWN_FRAMES
                eudkit.MinimapPing(REAR_ZONE)
                eudkit.DisplayText("Warning: enemy Zergling in the Rear Zone!")

    return {"watch_cd": watch_cd, "gate_cd": gate_cd, "rear_cd": rear_cd}


def main():
    session = eudkit.Session(BASEMAP_PATH)
    build(session)
    out_path = Path(__file__).parent / "output_v2.scx"
    report = session.compile(out_path)
    if out_path.exists():
        out_path.unlink()
    print("Scenario 41 (v2) compiled successfully:")
    print(f"  Triggers: {report.trigger_count}")
    print(f"  Payload:  {report.payload_size} bytes")
    print(f"  VProcs:   {report.vproc_count}")
    return report


if __name__ == "__main__":
    main()
