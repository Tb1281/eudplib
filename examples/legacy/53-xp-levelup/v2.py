# 53 - experience / level-up (modern eudkit)
#
# Expresses ideal intent:
# 1. Direct Deaths table reads via `dwread` with sub-dword EPD indexing.
# 2. Hero level-up progression with per-level threshold evaluation.
# 3. Clean local session state with zero global singletons.
import eudkit


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

from eudkit import session

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
    global level, xp_in_level, total_xp, level_ups
    global kills_zergling, kills_hydralisk, kills_ultralisk
    global prev_z, prev_h, prev_u

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
