# 56 - wave defense (modern eudkit)
#
# Expresses ideal intent:
# 1. Structured two-phase state machine (Intermission vs Assault).
# 2. Native python conditionals and control flow without scratch variables.
# 3. Direct memory reads for Deaths deltas with sub-dword EPD indexing.
from pathlib import Path
import eudkit

BASEMAP_PATH = Path(__file__).resolve().parents[3] / "tests" / "outputmap" / "basemap" / "basemap_strx.scx"

P1 = 0
P8 = 7
ZERGLING = 37
HYDRALISK = 38
LOC_SPAWN = 1
LOC_BASE = 2

MAX_WAVE = 8
INTERMISSION = 120
SPAWN_PERIOD = 6
START_LIVES = 10
CLEAR_ORE = 150
BONUS_GAS = 75
BONUS_EVERY = 3

DEATHS_ZERGLING = 0x58A364 + 4 * P8 + 48 * ZERGLING
DEATHS_HYDRALISK = 0x58A364 + 4 * P8 + 48 * HYDRALISK


def build(session: eudkit.Session):
    state = session.var(0)          # 0 = intermission, 1 = assault
    wave = session.var(1)
    inter_timer = session.var(0)
    spawn_timer = session.var(0)
    spawn_left = session.var(0)
    alive = session.var(0)
    spawn_alt = session.var(0)      # 0 = zergling, 1 = hydralisk
    lives = session.var(START_LIVES)
    victory = session.var(0)
    defeat = session.var(0)
    waves_cleared = session.var(0)
    bonus_counter = session.var(0)
    total_spawned = session.var(0)
    leaks = session.var(0)
    prev_zergling = session.var(0)
    prev_hydralisk = session.var(0)

    @session.fn(main_loop=True)
    def game_loop() -> None:
        nonlocal state, wave, inter_timer, spawn_timer, spawn_left, alive
        nonlocal spawn_alt, lives, victory, defeat, waves_cleared
        nonlocal bonus_counter, total_spawned, leaks, prev_zergling, prev_hydralisk

        if victory == 0 and defeat == 0:
            cur_z = eudkit.dwread(DEATHS_ZERGLING)
            cur_h = eudkit.dwread(DEATHS_HYDRALISK)
            if cur_z > prev_zergling:
                alive -= cur_z - prev_zergling
            if cur_h > prev_hydralisk:
                alive -= cur_h - prev_hydralisk
            prev_zergling = cur_z
            prev_hydralisk = cur_h

            if state == 1:
                if eudkit.Bring(P8, eudkit.AtLeast, 1, ZERGLING, LOC_BASE):
                    eudkit.KillUnitAt(1, ZERGLING, LOC_BASE, P8)
                    lives -= 1
                    leaks += 1
                if eudkit.Bring(P8, eudkit.AtLeast, 1, HYDRALISK, LOC_BASE):
                    eudkit.KillUnitAt(1, HYDRALISK, LOC_BASE, P8)
                    lives -= 1
                    leaks += 1
                if lives == 0:
                    defeat = 1

            if state == 0:
                inter_timer += 1
                if inter_timer >= INTERMISSION:
                    inter_timer = 0
                    spawn_left = 6 + wave * 3
                    spawn_timer = 0
                    state = 1
                    eudkit.MinimapPing(LOC_SPAWN)

            if state == 1:
                if spawn_left >= 1:
                    spawn_timer += 1
                    if spawn_timer >= SPAWN_PERIOD:
                        spawn_timer = 0
                        spawn_left -= 1
                        alive += 1
                        total_spawned += 1
                        if spawn_alt == 0:
                            eudkit.CreateUnit(1, ZERGLING, LOC_SPAWN, P8)
                            spawn_alt = 1
                        else:
                            eudkit.CreateUnit(1, HYDRALISK, LOC_SPAWN, P8)
                            spawn_alt = 0

                if spawn_left == 0 and alive == 0:
                    eudkit.SetResources(P1, eudkit.Add, CLEAR_ORE, eudkit.Ore)
                    waves_cleared += 1
                    bonus_counter += 1
                    if bonus_counter >= BONUS_EVERY:
                        bonus_counter = 0
                        eudkit.SetResources(P1, eudkit.Add, BONUS_GAS, eudkit.Gas)
                    if wave >= MAX_WAVE:
                        victory = 1
                    else:
                        wave += 1
                        state = 0
                        inter_timer = 0

    return {
        "state": state,
        "wave": wave,
        "alive": alive,
        "lives": lives,
        "victory": victory,
        "defeat": defeat,
    }


def main():
    session = eudkit.Session(BASEMAP_PATH)
    build(session)
    out_path = Path(__file__).parent / "output_v2.scx"
    report = session.compile(out_path)
    if out_path.exists():
        out_path.unlink()
    print("Scenario 56 (v2) compiled successfully:")
    print(f"  Triggers: {report.trigger_count}")
    print(f"  Payload:  {report.payload_size} bytes")
    print(f"  VProcs:   {report.vproc_count}")
    return report


if __name__ == "__main__":
    main()
