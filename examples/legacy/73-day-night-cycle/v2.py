# 73 - day / night cycle (modern eudkit)
#
# Expresses ideal intent:
# 1. State machine managing daytime and nighttime phases.
# 2. Weapons.dat live damage table patching via `wwrite`.
# 3. Deduplicated map strings and periodic spawn waves via `with session.every(100):`.
from pathlib import Path
import eudkit

BASEMAP_PATH = Path(__file__).resolve().parents[3] / "tests" / "outputmap" / "basemap" / "basemap_strx.scx"

ZERGLING = 37
HYDRALISK = 38
CLAWS = 35
WEAPON_DMG_BASE = 0x656EB0
ENEMY = 7
ENEMY_BASE = 50
PHASE_LEN = 600
SPAWN_PERIOD = 100
CLAWS_DAY = 5
CLAWS_NIGHT = 8


def build(session: eudkit.Session):
    phase_tick = session.var(0)
    phase = session.var(0)          # 0 day, 1 night
    prev_phase = session.var(0)
    day_spawns = session.var(0)
    night_spawns = session.var(0)
    cycles = session.var(0)
    transitions = session.var(0)

    msg_night = session.map_string("Night falls. The predators grow bold.")
    msg_dawn = session.map_string("Dawn breaks. The swarm rests.")

    @session.fn(main_loop=True)
    def game_loop() -> None:
        nonlocal phase_tick, phase, prev_phase, day_spawns, night_spawns
        nonlocal cycles, transitions

        phase_tick += 1
        if phase_tick >= PHASE_LEN:
            phase_tick = 0
            if phase == 0:
                phase = 1
            else:
                phase = 0

        if phase != prev_phase:
            transitions += 1
            if phase == 1:
                eudkit.wwrite(WEAPON_DMG_BASE + 2 * CLAWS, CLAWS_NIGHT)
                msg_night.display()
            else:
                eudkit.wwrite(WEAPON_DMG_BASE + 2 * CLAWS, CLAWS_DAY)
                cycles += 1
                msg_dawn.display()
            prev_phase = phase

        with session.every(SPAWN_PERIOD):
            if phase == 0:
                eudkit.CreateUnit(2, ZERGLING, ENEMY_BASE, ENEMY)
                day_spawns += 2
            else:
                eudkit.CreateUnit(1, HYDRALISK, ENEMY_BASE, ENEMY)
                night_spawns += 1

    return {
        "phase": phase,
        "cycles": cycles,
        "transitions": transitions,
        "day_spawns": day_spawns,
        "night_spawns": night_spawns,
    }


def main():
    session = eudkit.Session(BASEMAP_PATH)
    build(session)
    out_path = Path(__file__).parent / "output_v2.scx"
    report = session.compile(out_path)
    if out_path.exists():
        out_path.unlink()
    print("Scenario 73 (v2) compiled successfully:")
    print(f"  Triggers: {report.trigger_count}")
    print(f"  Payload:  {report.payload_size} bytes")
    print(f"  VProcs:   {report.vproc_count}")
    return report


if __name__ == "__main__":
    main()
