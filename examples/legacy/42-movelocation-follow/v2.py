# 42 - MoveLocation follow pattern (modern eudkit)
#
# Expresses ideal intent:
# 1. Native `MoveLocation` action recentering tracking zone every frame.
# 2. `with once():` eliminates 3 manual boolean state variables.
# 3. Clean local session state with zero VProcs.
from pathlib import Path
import eudkit

BASEMAP_PATH = Path(__file__).resolve().parents[3] / "tests" / "outputmap" / "basemap" / "basemap_strx.scx"

MARINE = 0
ZERGLING = 37
HYDRALISK = 38

TRACK_ZONE = 1
BASE_ZONE = 2
ANYWHERE = 0


def build(session: eudkit.Session):
    score_cd = session.var(0)

    @session.fn(main_loop=True)
    def game_loop() -> None:
        nonlocal score_cd
        # Recenter track zone on player's marine
        eudkit.MoveLocation(TRACK_ZONE, MARINE, eudkit.CurrentPlayer, ANYWHERE)

        if score_cd >= 1:
            score_cd -= 1

        if eudkit.Bring(eudkit.CurrentPlayer, eudkit.AtLeast, 3, ZERGLING, TRACK_ZONE):
            with eudkit.once():
                eudkit.DisplayText("Contact! Zerglings on our Marine.")
            if score_cd == 0:
                score_cd = 84
                eudkit.SetScore(eudkit.CurrentPlayer, eudkit.Add, 10, eudkit.Custom)

        if eudkit.Bring(eudkit.P2, eudkit.AtLeast, 1, HYDRALISK, BASE_ZONE):
            with eudkit.once():
                eudkit.MinimapPing(BASE_ZONE)
                eudkit.DisplayText("A Hydralisk slipped into the Base Zone!")

        if eudkit.Bring(eudkit.P2, eudkit.AtLeast, 5, ZERGLING, BASE_ZONE):
            with eudkit.once():
                eudkit.MinimapPing(BASE_ZONE)
                eudkit.DisplayText("The base is being overrun!")

    return {"score_cd": score_cd}


def main():
    session = eudkit.Session(BASEMAP_PATH)
    build(session)
    out_path = Path(__file__).parent / "output_v2.scx"
    report = session.compile(out_path)
    if out_path.exists():
        out_path.unlink()
    print("Scenario 42 (v2) compiled successfully:")
    print(f"  Triggers: {report.trigger_count}")
    print(f"  Payload:  {report.payload_size} bytes")
    print(f"  VProcs:   {report.vproc_count}")
    return report


if __name__ == "__main__":
    main()
