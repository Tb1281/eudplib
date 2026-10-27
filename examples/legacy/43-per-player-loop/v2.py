# 43 - per-player round bonus + personal message (modern eudkit)
#
# Expresses ideal intent:
# 1. Reactive `with every(252):` clock replaces manual frame counting and reset logic.
# 2. Localized player messaging via `IsUserCP()` with zero EUDFunc trampoline overhead.
# 3. Clean local session state with zero VProcs.
from pathlib import Path
import eudkit

BASEMAP_PATH = Path(__file__).resolve().parents[3] / "tests" / "outputmap" / "basemap" / "basemap_strx.scx"

MARINE = 0
MARINES_REQUIRED = 3
BONUS_ORE = 25
ELITE_MARINES = 10
ELITE_GAS = 10
ROUND_INTERVAL = 252    # ~15 s at Fastest


def build(session: eudkit.Session):
    @session.fn(main_loop=True)
    def game_loop() -> None:
        with eudkit.every(ROUND_INTERVAL):
            for player in range(8):
                if eudkit.Command(player, eudkit.AtLeast, MARINES_REQUIRED, MARINE):
                    eudkit.SetResources(player, eudkit.Add, BONUS_ORE, eudkit.Ore)
                    eudkit.dwwrite(0x6509B0, player)
                    if eudkit.IsUserCP():
                        eudkit.DisplayText("Marine corps bonus paid: +25 ore.")

                    if eudkit.Command(player, eudkit.AtLeast, ELITE_MARINES, MARINE):
                        eudkit.SetResources(player, eudkit.Add, ELITE_GAS, eudkit.Gas)
                        eudkit.dwwrite(0x6509B0, player)
                        if eudkit.IsUserCP():
                            eudkit.DisplayText("Elite corps bonus paid: +10 gas.")


def main():
    session = eudkit.Session(BASEMAP_PATH)
    build(session)
    out_path = Path(__file__).parent / "output_v2.scx"
    report = session.compile(out_path)
    if out_path.exists():
        out_path.unlink()
    print("Scenario 43 (v2) compiled successfully:")
    print(f"  Triggers: {report.trigger_count}")
    print(f"  Payload:  {report.payload_size} bytes")
    print(f"  VProcs:   {report.vproc_count}")
    return report


if __name__ == "__main__":
    main()
