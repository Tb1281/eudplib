# 39 - research completion detection (modern eudkit)
#
# Expresses ideal intent:
# 1. Direct byte reads on research table via `bread(...)` with automatic EPD and byte masking.
# 2. Declarative `with once():` eliminates 8 manual boolean guard variables.
# 3. Clean local session state with zero global singletons.
from pathlib import Path
import eudkit

BASEMAP_PATH = Path(__file__).resolve().parents[3] / "tests" / "outputmap" / "basemap" / "basemap_strx.scx"

RESEARCHED_BASE = 0x58CF44
STIM_PACKS = 0
LOCKDOWN = 1
BONUS_GAS = 100
LOCKDOWN_BONUS_ORE = 50


def build(session: eudkit.Session):
    @session.fn(main_loop=True)
    def game_loop() -> None:
        for player in range(4):
            stim_state = eudkit.bread(RESEARCHED_BASE + player * 24 + STIM_PACKS)
            if stim_state != 0:
                with eudkit.once():
                    eudkit.SetResources(player, eudkit.Add, BONUS_GAS, eudkit.Gas)
                    eudkit.DisplayText("Stim Packs research complete! +100 gas.")

            lockdown_state = eudkit.bread(RESEARCHED_BASE + player * 24 + LOCKDOWN)
            if lockdown_state != 0:
                with eudkit.once():
                    eudkit.SetResources(player, eudkit.Add, LOCKDOWN_BONUS_ORE, eudkit.Ore)
                    eudkit.DisplayText("Lockdown research complete! +50 ore.")


def main():
    session = eudkit.Session(BASEMAP_PATH)
    build(session)
    out_path = Path(__file__).parent / "output_v2.scx"
    report = session.compile(out_path)
    if out_path.exists():
        out_path.unlink()
    print("Scenario 39 (v2) compiled successfully:")
    print(f"  Triggers: {report.trigger_count}")
    print(f"  Payload:  {report.payload_size} bytes")
    print(f"  VProcs:   {report.vproc_count}")
    return report


if __name__ == "__main__":
    main()
