# 57 - tower upgrade (modern eudkit)
#
# Expresses ideal intent:
# 1. Clean multi-level tower upgrade progression with structured branch dispatch.
# 2. Native shot firing loop with dynamic target presence evaluation.
# 3. Direct memory reads for Deaths deltas with sub-dword EPD indexing.
from pathlib import Path
import eudkit

BASEMAP_PATH = Path(__file__).resolve().parents[3] / "tests" / "outputmap" / "basemap" / "basemap_strx.scx"

P1 = 0
P8 = 7
ZERGLING = 37
HYDRALISK = 38
LOC_TOWER = 3

MAX_LEVEL = 3
BASE_COOLDOWN = 40
CD_PER_LEVEL = 5
UPGRADE_COSTS = [150, 250, 400]
SW_UPGRADE = 7

DEATHS_ZERGLING = 0x58A364 + 4 * P8 + 48 * ZERGLING
DEATHS_HYDRALISK = 0x58A364 + 4 * P8 + 48 * HYDRALISK


def build(session: eudkit.Session):
    tower_level = session.var(0)    # 0..3
    cooldown = session.var(0)
    fired = session.var(0)
    shots_fired = session.var(0)
    tower_kills = session.var(0)
    upgrades = session.var(0)
    failed_buys = session.var(0)
    ore_spent = session.var(0)
    prev_zergling = session.var(0)
    prev_hydralisk = session.var(0)

    @session.fn(main_loop=True)
    def game_loop() -> None:
        nonlocal tower_level, cooldown, fired, shots_fired, tower_kills
        nonlocal upgrades, failed_buys, ore_spent, prev_zergling, prev_hydralisk

        cur_z = eudkit.dwread(DEATHS_ZERGLING)
        cur_h = eudkit.dwread(DEATHS_HYDRALISK)
        if cur_z > prev_zergling:
            tower_kills += cur_z - prev_zergling
        if cur_h > prev_hydralisk:
            tower_kills += cur_h - prev_hydralisk
        prev_zergling = cur_z
        prev_hydralisk = cur_h

        if eudkit.Switch(SW_UPGRADE, eudkit.Set):
            eudkit.SetSwitch(SW_UPGRADE, eudkit.Clear)
            if tower_level == 0:
                if eudkit.Accumulate(P1, eudkit.AtLeast, UPGRADE_COSTS[0], eudkit.Ore):
                    eudkit.SetResources(P1, eudkit.Subtract, UPGRADE_COSTS[0], eudkit.Ore)
                    tower_level += 1
                    upgrades += 1
                    ore_spent += UPGRADE_COSTS[0]
                    eudkit.MinimapPing(LOC_TOWER)
                else:
                    failed_buys += 1
            elif tower_level == 1:
                if eudkit.Accumulate(P1, eudkit.AtLeast, UPGRADE_COSTS[1], eudkit.Ore):
                    eudkit.SetResources(P1, eudkit.Subtract, UPGRADE_COSTS[1], eudkit.Ore)
                    tower_level += 1
                    upgrades += 1
                    ore_spent += UPGRADE_COSTS[1]
                    eudkit.MinimapPing(LOC_TOWER)
                else:
                    failed_buys += 1
            elif tower_level == 2:
                if eudkit.Accumulate(P1, eudkit.AtLeast, UPGRADE_COSTS[2], eudkit.Ore):
                    eudkit.SetResources(P1, eudkit.Subtract, UPGRADE_COSTS[2], eudkit.Ore)
                    tower_level += 1
                    upgrades += 1
                    ore_spent += UPGRADE_COSTS[2]
                    eudkit.MinimapPing(LOC_TOWER)
                else:
                    failed_buys += 1
            else:
                failed_buys += 1

        if cooldown >= 1:
            cooldown -= 1

        has_target = 0
        if eudkit.Bring(P8, eudkit.AtLeast, 1, ZERGLING, LOC_TOWER):
            has_target = 1
        if eudkit.Bring(P8, eudkit.AtLeast, 1, HYDRALISK, LOC_TOWER):
            has_target = 1

        if cooldown == 0 and has_target == 1:
            fired = 0
            while fired <= tower_level:
                if eudkit.Bring(P8, eudkit.AtLeast, 1, ZERGLING, LOC_TOWER):
                    eudkit.KillUnitAt(1, ZERGLING, LOC_TOWER, P8)
                    fired += 1
                    shots_fired += 1
                elif eudkit.Bring(P8, eudkit.AtLeast, 1, HYDRALISK, LOC_TOWER):
                    eudkit.KillUnitAt(1, HYDRALISK, LOC_TOWER, P8)
                    fired += 1
                    shots_fired += 1
                else:
                    fired = 99
            cooldown = BASE_COOLDOWN - tower_level * CD_PER_LEVEL

    return {
        "tower_level": tower_level,
        "cooldown": cooldown,
        "shots_fired": shots_fired,
        "tower_kills": tower_kills,
        "upgrades": upgrades,
        "failed_buys": failed_buys,
        "ore_spent": ore_spent,
    }


def main():
    session = eudkit.Session(BASEMAP_PATH)
    build(session)
    out_path = Path(__file__).parent / "output_v2.scx"
    report = session.compile(out_path)
    if out_path.exists():
        out_path.unlink()
    print("Scenario 57 (v2) compiled successfully:")
    print(f"  Triggers: {report.trigger_count}")
    print(f"  Payload:  {report.payload_size} bytes")
    print(f"  VProcs:   {report.vproc_count}")
    return report


if __name__ == "__main__":
    main()
