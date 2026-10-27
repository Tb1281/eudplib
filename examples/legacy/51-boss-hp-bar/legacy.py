# 51 - boss HP bar + phase bursts (legacy eudplib)
#
# Intent: a boss fight HUD. The boss is a var-tracked HP pool
# (5000 HP). While Player 1 holds at least 3 Marines in the boss
# zone (location 1), the boss takes 25 damage per trigger loop.
# Crossing 3000 HP (phase 2) and 1500 HP (phase 3) fires a one-time
# minion burst at the spawn point (location 2) with a minimap ping.
# At 0 HP the boss dies: +800 ore reward and a victory latch. Every
# 12 loops the HUD redraws a text bar: 10 segments, filled count =
# hp * 10 / 5000, followed by the numeric HP.
#
# Counting form (pilot convention): run() builds the scenario's
# triggers once; benchmarks/pilot/count_legacy.py calls it inside a
# fresh PushTriggerScope and reports the GetTriggerCounter() delta.
# In production the body is the beforeTriggerExec hook; run() is
# that body verbatim. Units and locations are numeric ids and the
# bar text lives in a DBString (no basemap string lookup in the
# harness, exactly like the pilot scenarios); a real map build
# resolves the same calls by name and creates identical triggers.
# LoadMap is module level: DBString.Display() allocates a string id
# from the loaded map, as in any real legacy build.
from eudkit import *
from eudkit.string.dbstr import DBString

LoadMap("tests/outputmap/basemap/basemap.scx")

P1 = 0
P8 = 7
MARINE = 0          # unit ids
ZERGLING = 37
HYDRALISK = 38
LOC_BOSS = 1        # location ids
LOC_SPAWN = 2

BOSS_MAX_HP = 5000
DAMAGE = 25
PHASE2_HP = 3000
PHASE3_HP = 1500
REWARD_ORE = 800
BAR_PERIOD = 12     # trigger loops between HUD redraws
BAR_SEGMENTS = 10

# Precomputed bar bodies, one per filled count 0..10 -- the classic
# legacy approach to a progress bar (no run-time string building).
BAR_STRINGS = [
    "----------",
    "#---------",
    "##--------",
    "###-------",
    "####------",
    "#####-----",
    "######----",
    "#######---",
    "########--",
    "#########-",
    "##########",
]

boss_hp = EUDVariable(BOSS_MAX_HP)  # remaining boss HP
phase = EUDVariable(1)              # 1/2/3 while alive
victory = EUDVariable(0)            # latched on boss death
filled = EUDVariable(0)             # scratch: filled segments 0..10
bursts = EUDVariable(0)             # lifetime phase bursts fired
renders = EUDVariable(0)            # lifetime HUD redraws
bar_tick = EUDVariable(0)           # loops since the last redraw

bar_buf = DBString(64)              # the one reusable HUD buffer


def run():
    global boss_hp, phase, victory, filled, bursts, renders, bar_tick
    if EUDIf()(victory == 0):
        # Damage: the marine squad grinds the boss down.
        if EUDIf()(Bring(P1, AtLeast, 3, MARINE, LOC_BOSS)):
            if EUDIf()(boss_hp >= DAMAGE):
                boss_hp -= DAMAGE
            EUDEndIf()
        EUDEndIf()

        # Phase 2 burst (one-time, guarded by the phase var itself).
        if EUDIf()(phase == 1):
            if EUDIf()(boss_hp <= PHASE2_HP):
                if EUDIf()(boss_hp >= 1):
                    phase << 2
                    DoActions([
                        CreateUnit(4, ZERGLING, LOC_SPAWN, P8),
                        MinimapPing(LOC_SPAWN),
                    ])
                    bursts += 1
                EUDEndIf()
            EUDEndIf()
        EUDEndIf()

        # Phase 3 burst: zerglings plus hydralisks.
        if EUDIf()(phase == 2):
            if EUDIf()(boss_hp <= PHASE3_HP):
                if EUDIf()(boss_hp >= 1):
                    phase << 3
                    DoActions([
                        CreateUnit(4, ZERGLING, LOC_SPAWN, P8),
                        CreateUnit(2, HYDRALISK, LOC_SPAWN, P8),
                        MinimapPing(LOC_SPAWN),
                    ])
                    bursts += 1
                EUDEndIf()
            EUDEndIf()
        EUDEndIf()

        # Death: reward and latch.
        if EUDIf()(boss_hp == 0):
            victory << 1
            DoActions([
                SetResources(P1, Add, REWARD_ORE, Ore),
                MinimapPing(LOC_BOSS),
            ])
        EUDEndIf()
    EUDEndIf()

    # HUD refresh on its own period (hand-rolled timer).
    bar_tick += 1
    if EUDIf()(bar_tick >= BAR_PERIOD):
        bar_tick << 0
        filled, _remainder = f_div(boss_hp * BAR_SEGMENTS, BOSS_MAX_HP)
        # Equality dispatch over the 11 precomputed bar bodies.
        for i in range(BAR_SEGMENTS + 1):
            if EUDIf()(filled == i):
                f_dbstr_print(bar_buf, "BOSS [" + BAR_STRINGS[i] + "] ",
                              boss_hp)
            EUDEndIf()
        bar_buf.Display()
        renders += 1
    EUDEndIf()
