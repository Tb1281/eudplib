# 53 - experience / level-up (modern eudkit)
#
# Expresses ideal intent:
# 1. Direct Deaths table reads via `dwread` with sub-dword EPD indexing.
# 2. Hero level-up progression with per-level threshold evaluation.
# 3. Clean local session state with zero global singletons.
from pathlib import Path
import eudkit

BASEMAP_PATH = Path(__file__).resolve().parents[3] / "tests" / "outputmap" / "basemap" / "basemap_strx.scx"

P1 = 0
P8 = 7
LOC_BASE = 1

ZERGLING = 37
HYDRALISK = 38
ULTRALISK = 39
XP_ZERGLING = 5
XP_HYDRALISK = 15
XP_ULTRALISK = 40
MAX_LEVEL = 10
LEVEL_ORE = 150
LEVEL_GAS = 25

DEATHS_ZERGLING = 0x58A364 + 4 * P8 + 48 * ZERGLING
DEATHS_HYDRALISK = 0x58A364 + 4 * P8 + 48 * HYDRALISK
DEATHS_ULTRALISK = 0x58A364 + 4 * P8 + 48 * ULTRALISK


def need_for(l: int) -> int:
    return 100 + 20 * (l - 1)


def build(session: eudkit.Session):
    level = session.var(1)
    xp_in_level = session.var(0)
    total_xp = session.var(0)
    level_ups = session.var(0)
    kills_zergling = session.var(0)
    kills_hydralisk = session.var(0)
    kills_ultralisk = session.var(0)
    prev_z = session.var(0)
    prev_h = session.var(0)
    prev_u = session.var(0)

    @session.fn(main_loop=True)
    def game_loop() -> None:
        nonlocal level, xp_in_level, total_xp, level_ups
        nonlocal kills_zergling, kills_hydralisk, kills_ultralisk
        nonlocal prev_z, prev_h, prev_u

        cur_z = eudkit.dwread(DEATHS_ZERGLING)
        cur_h = eudkit.dwread(DEATHS_HYDRALISK)
        cur_u = eudkit.dwread(DEATHS_ULTRALISK)

        gained = 0
        if cur_z > prev_z:
            dz = cur_z - prev_z
            gained += dz * XP_ZERGLING
            kills_zergling += dz
        if cur_h > prev_h:
            dh = cur_h - prev_h
            gained += dh * XP_HYDRALISK
            kills_hydralisk += dh
        if cur_u > prev_u:
            du = cur_u - prev_u
            gained += du * XP_ULTRALISK
            kills_ultralisk += du

        prev_z = cur_z
        prev_h = cur_h
        prev_u = cur_u

        if gained >= 1:
            xp_in_level += gained
            total_xp += gained

        for l in range(1, MAX_LEVEL):
            req = need_for(l)
            if level == l:
                if xp_in_level >= req:
                    xp_in_level -= req
                    level += 1
                    level_ups += 1
                    eudkit.SetResources(P1, eudkit.Add, LEVEL_ORE, eudkit.Ore)
                    eudkit.SetResources(P1, eudkit.Add, LEVEL_GAS, eudkit.Gas)
                    eudkit.MinimapPing(LOC_BASE)

    return {
        "level": level,
        "xp_in_level": xp_in_level,
        "total_xp": total_xp,
        "level_ups": level_ups,
    }


def main():
    session = eudkit.Session(BASEMAP_PATH)
    build(session)
    out_path = Path(__file__).parent / "output_v2.scx"
    report = session.compile(out_path)
    if out_path.exists():
        out_path.unlink()
    print("Scenario 53 (v2) compiled successfully:")
    print(f"  Triggers: {report.trigger_count}")
    print(f"  Payload:  {report.payload_size} bytes")
    print(f"  VProcs:   {report.vproc_count}")
    return report


if __name__ == "__main__":
    main()
