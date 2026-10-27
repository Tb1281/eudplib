# 37 - Deaths-table kill ledger + score (modern eudkit)
#
# Expresses ideal intent:
# 1. Native Deaths table ledger and Score trigger actions.
# 2. Declarative one-time announcement via `with once():` replacing
#    manual boolean flag variables and multi-frame guard checks.
# 3. Clean local session state.
from pathlib import Path
import eudkit

BASEMAP_PATH = Path(__file__).resolve().parents[3] / "tests" / "outputmap" / "basemap" / "basemap_strx.scx"

MARINE = 0
ZERGLING = 37
HYDRALISK = 38
ULTRALISK = 39


def build(session: eudkit.Session):
    @session.fn(main_loop=True)
    def game_loop() -> None:
        if eudkit.Deaths(eudkit.CurrentPlayer, eudkit.Exactly, 1, ZERGLING):
            eudkit.SetDeaths(eudkit.CurrentPlayer, eudkit.Add, 1, MARINE)
            eudkit.SetScore(eudkit.CurrentPlayer, eudkit.Add, 100, eudkit.Custom)

        if eudkit.Deaths(eudkit.CurrentPlayer, eudkit.Exactly, 1, HYDRALISK):
            eudkit.SetDeaths(eudkit.CurrentPlayer, eudkit.Add, 2, MARINE)
            eudkit.SetScore(eudkit.CurrentPlayer, eudkit.Add, 250, eudkit.Custom)

        if eudkit.Deaths(eudkit.CurrentPlayer, eudkit.Exactly, 1, ULTRALISK):
            eudkit.SetDeaths(eudkit.CurrentPlayer, eudkit.Add, 5, MARINE)
            eudkit.SetScore(eudkit.CurrentPlayer, eudkit.Add, 500, eudkit.Custom)
            eudkit.DisplayText("Ultralisk down! +500 score.")

        if eudkit.Deaths(eudkit.CurrentPlayer, eudkit.AtLeast, 5, MARINE):
            eudkit.SetResources(eudkit.CurrentPlayer, eudkit.Add, 200, eudkit.Ore)
            eudkit.SetDeaths(eudkit.CurrentPlayer, eudkit.SetTo, 0, MARINE)
            eudkit.DisplayText("Kill bonus paid: +200 ore.")

        if eudkit.Score(eudkit.CurrentPlayer, eudkit.Custom, eudkit.AtLeast, 1000):
            with eudkit.once():
                eudkit.DisplayText("Veteran! Score passed 1000.")


def main():
    session = eudkit.Session(BASEMAP_PATH)
    build(session)
    out_path = Path(__file__).parent / "output_v2.scx"
    report = session.compile(out_path)
    if out_path.exists():
        out_path.unlink()
    print("Scenario 37 (v2) compiled successfully:")
    print(f"  Triggers: {report.trigger_count}")
    print(f"  Payload:  {report.payload_size} bytes")
    print(f"  VProcs:   {report.vproc_count}")
    return report


if __name__ == "__main__":
    main()
