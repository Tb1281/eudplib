"""Example 06: Reactive Rules & Events — Legacy eudplib (v1).

Demonstrates:
- Manual timer variables (EUDVariable tick decrements every frame)
- One-shot boolean flags (init_once = 1 -> set to 0)
- Fragmented state machine triggers to simulate 'after', 'every', 'when', 'once'
- Risk of trigger desynchronization with StarCraft Wait() actions
"""

from pathlib import Path
import eudplib as ep

REPO_ROOT = Path(__file__).resolve().parent.parent.parent
BASEMAP_PATH = REPO_ROOT / "tests" / "outputmap" / "basemap" / "basemap_strx.scx"


def main():
    ep.LoadMap(str(BASEMAP_PATH))
    ep.CompressPayload(True)

    def v1_reactive_workload():
        # 1. State machine for 'once' initialization
        init_once = ep.EUDVariable(1)
        if ep.EUDIf()(init_once == 1):
            init_once << 0
            ep.f_eprintln("Welcome to StarCraft EUD!")
            ep.DoActions(ep.SetResources(ep.Player1, ep.SetTo, 1000, ep.Ore))
        ep.EUDEndIf()

        # 2. State machine for 'every' 24 ticks (1 second)
        timer_24 = ep.EUDVariable(24)
        if ep.EUDIf()(timer_24 > 0):
            timer_24 -= 1
        ep.EUDEndIf()
        if ep.EUDIf()(timer_24 == 0):
            timer_24 << 24
            ep.DoActions(ep.SetResources(ep.Player1, ep.Add, 50, ep.Ore))
        ep.EUDEndIf()

        # 3. State machine for 'after' 120 ticks
        delayed_timer = ep.EUDVariable(120)
        delayed_done = ep.EUDVariable(0)
        if ep.EUDIf()([delayed_done == 0, delayed_timer > 0]):
            delayed_timer -= 1
        ep.EUDEndIf()
        if ep.EUDIf()([delayed_done == 0, delayed_timer == 0]):
            delayed_done << 1
            ep.f_eprintln("Reinforcements have arrived!")
            ep.DoActions(ep.CreateUnit(1, "Terran Marine", "Anywhere", ep.Player1))
        ep.EUDEndIf()

    out_path = Path(__file__).parent / "output_v1.scx"
    ep.SaveMap(str(out_path), v1_reactive_workload)
    print(f"Legacy v1 reactive workload compiled to: {out_path}")


if __name__ == "__main__":
    main()
