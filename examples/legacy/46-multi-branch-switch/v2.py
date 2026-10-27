# 46 - multi-branch switch dispatch (modern eudkit)
#
# Expresses ideal intent:
# 1. Native Python if/elif/else control flow for state machine dispatch.
# 2. Scoped session variables with zero global singletons.
# 3. Direct stock actions for unit creation, victory, and defeat.
from pathlib import Path
import eudkit

BASEMAP_PATH = Path(__file__).resolve().parents[3] / "tests" / "outputmap" / "basemap" / "basemap_strx.scx"

PH_IDLE = 0
PH_COUNTDOWN = 1
PH_WAVE_LIGHT = 2
PH_WAVE_HEAVY = 3
PH_BOSS = 4
PH_SHOP = 5
PH_VICTORY = 6
PH_DEFEAT = 7

MARINE = 0
ZERGLING = 37
HYDRALISK = 38
ULTRALISK = 39
COMMAND_CENTER = 106
ANYWHERE = 0


def build(session: eudkit.Session):
    phase = session.var(0)
    wave_timer = session.var(0)
    wave_number = session.var(0)
    spawn_left = session.var(0)
    gold = session.var(0)
    kills = session.var(0)

    @session.fn(main_loop=True)
    def game_loop() -> None:
        nonlocal phase, wave_timer, wave_number, spawn_left, gold, kills

        if phase == PH_IDLE:
            if eudkit.Deaths(eudkit.CurrentPlayer, eudkit.AtLeast, 1, MARINE):
                phase = PH_COUNTDOWN
                wave_timer = 72
        elif phase == PH_COUNTDOWN:
            wave_timer -= 1
            if wave_timer == 0:
                wave_number += 1
                spawn_left = 10 + wave_number * 2
                phase = PH_WAVE_LIGHT
        elif phase == PH_WAVE_LIGHT:
            if spawn_left > 0:
                eudkit.CreateUnit(1, ZERGLING, ANYWHERE, eudkit.P8)
                spawn_left -= 1
            if spawn_left == 0:
                phase = PH_WAVE_HEAVY
                spawn_left = wave_number
        elif phase == PH_WAVE_HEAVY:
            if spawn_left > 0:
                eudkit.CreateUnit(1, HYDRALISK, ANYWHERE, eudkit.P8)
                spawn_left -= 1
            if spawn_left == 0:
                if wave_number >= 5:
                    phase = PH_BOSS
                else:
                    phase = PH_SHOP
        elif phase == PH_BOSS:
            if eudkit.Deaths(eudkit.P8, eudkit.AtLeast, 1, ULTRALISK):
                gold += 500
                phase = PH_SHOP
        elif phase == PH_SHOP:
            wave_timer = 240
            phase = PH_COUNTDOWN
        elif phase == PH_VICTORY:
            eudkit.DisplayText("Victory!")
            eudkit.Victory()
        elif phase == PH_DEFEAT:
            eudkit.DisplayText("Defeat...")
            eudkit.Defeat()
        else:
            phase = PH_IDLE

        # Victory/defeat preempt the dispatch
        if kills >= 300:
            phase = PH_VICTORY
        if eudkit.Deaths(eudkit.CurrentPlayer, eudkit.AtLeast, 1, COMMAND_CENTER):
            phase = PH_DEFEAT

    return {
        "phase": phase,
        "wave_timer": wave_timer,
        "wave_number": wave_number,
        "gold": gold,
        "kills": kills,
    }


def main():
    session = eudkit.Session(BASEMAP_PATH)
    build(session)
    out_path = Path(__file__).parent / "output_v2.scx"
    report = session.compile(out_path)
    if out_path.exists():
        out_path.unlink()
    print("Scenario 46 (v2) compiled successfully:")
    print(f"  Triggers: {report.trigger_count}")
    print(f"  Payload:  {report.payload_size} bytes")
    print(f"  VProcs:   {report.vproc_count}")
    return report


if __name__ == "__main__":
    main()
