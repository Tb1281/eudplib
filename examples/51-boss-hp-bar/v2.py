# 51 - boss HP bar + phase bursts (modern eudkit)
#
# Expresses ideal intent:
# 1. Modern `StringBuffer` for periodic HUD redraws with static STRx relocation (ADR 0004).
# 2. Reactive `with every(12):` timer eliminates manual tick counter.
# 3. Clean local session state with zero global singletons.
import eudkit


P1 = 0
P8 = 7
MARINE = 0
ZERGLING = 37
HYDRALISK = 38
LOC_BOSS = 1
LOC_SPAWN = 2

BOSS_MAX_HP = 5000
DAMAGE = 25
PHASE2_HP = 3000
PHASE3_HP = 1500
REWARD_ORE = 800
BAR_PERIOD = 12
BAR_SEGMENTS = 10

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

from eudkit import session

boss_hp = session.var(BOSS_MAX_HP)
phase = session.var(1)
victory = session.var(0)
bursts = session.var(0)
renders = session.var(0)

bar_buf = session.string(64)

@session.fn(main_loop=True)
def game_loop() -> None:
    global boss_hp, phase, victory, bursts, renders

    if victory == 0:
        if eudkit.Bring(P1, eudkit.AtLeast, 3, MARINE, LOC_BOSS):
            if boss_hp >= DAMAGE:
                boss_hp -= DAMAGE

        if phase == 1:
            if boss_hp <= PHASE2_HP:
                if boss_hp >= 1:
                    phase = 2
                    eudkit.CreateUnit(4, ZERGLING, LOC_SPAWN, P8)
                    eudkit.MinimapPing(LOC_SPAWN)
                    bursts += 1

        if phase == 2:
            if boss_hp <= PHASE3_HP:
                if boss_hp >= 1:
                    phase = 3
                    eudkit.CreateUnit(4, ZERGLING, LOC_SPAWN, P8)
                    eudkit.CreateUnit(2, HYDRALISK, LOC_SPAWN, P8)
                    eudkit.MinimapPing(LOC_SPAWN)
                    bursts += 1

        if boss_hp == 0:
            victory = 1
            eudkit.SetResources(P1, eudkit.Add, REWARD_ORE, eudkit.Ore)
            eudkit.MinimapPing(LOC_BOSS)

    with eudkit.every(BAR_PERIOD):
        filled = (boss_hp * BAR_SEGMENTS) // BOSS_MAX_HP
        for i in range(BAR_SEGMENTS + 1):
            if filled == i:
                bar_buf.printf("BOSS [" + BAR_STRINGS[i] + "] %d", boss_hp)
        bar_buf.display_text()
        renders += 1
