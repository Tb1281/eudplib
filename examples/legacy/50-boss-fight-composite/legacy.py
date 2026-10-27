"""LEGACY boss fight composite (benchmark 50).

Same intent as ours.js: everything combined -- a boss encounter
driven by one per-frame body:

  * start signal (Marine deaths) spawns the boss (Ultralisk, P8)
  * a hand-rolled 24-frame timer spawns 2 Zergling adds
  * a hit register (boss_hits, written by the combat system) drains
    boss_hp by 25 per hit
  * crossing 25% HP is a hand-rolled rising edge: enrage -- phase 2
    plus 4 Hydralisk escorts, once
  * an HP bar in a StringBuffer, rebuilt only when HP changed and
    displayed every frame while the boss is up
  * boss_hp == 0: victory -- +500 ore, phase 3, and after a
    48-frame cleanup timer everything resets to idle
"""

from eudkit import *

BOSS_HP_MAX = 1000
ENRAGE_AT = 250
HIT_DAMAGE = 25
ORE_EPD = EPD(0x57F0F0)  # Player1 ore dword, as a Deaths EPD

phase = EUDVariable(0)  # 0 idle, 1 fight, 2 enraged, 3 victory cleanup
boss_hp = EUDVariable(0)
boss_hits = EUDVariable(0)       # hit register (combat system writes)
applied_hits = EUDVariable(0)    # hits already drained into boss_hp
gold = EUDVariable(0)
adds_spawned = EUDVariable(0)
spawn_timer = EUDVariable(0)
prev_enrage = EUDVariable(0)
shown_hp = EUDVariable(0xFFFFFFFF)
cleanup_timer = EUDVariable(0)
bar_fill = EUDVariable(0)
bar_k = EUDVariable(0)

bar = StringBuffer(96)


def run():
    global phase, boss_hp, applied_hits, gold, adds_spawned
    global spawn_timer, prev_enrage, shown_hp, cleanup_timer
    global bar_fill, bar_k
    # 1. Start signal: first Marine death while idle spawns the boss.
    if EUDIf()(Deaths(CurrentPlayer, AtLeast, 1, "Terran Marine")):
        if EUDIf()(phase == 0):
            phase << 1
            boss_hp << BOSS_HP_MAX
            DoActions(CreateUnit(1, "Zerg Ultralisk", "Anywhere", P8))
        EUDEndIf()
    EUDEndIf()

    # 2. Add spawner: hand-rolled every-24-frames timer.
    if EUDIf()(phase == 1):
        spawn_timer += 1
        if EUDIf()(spawn_timer >= 24):
            spawn_timer << 0
            DoActions(CreateUnit(2, "Zerg Zergling", "Anywhere", P8))
            adds_spawned += 2
        EUDEndIf()
    EUDEndIf()
    if EUDIf()(phase == 2):
        spawn_timer += 1
        if EUDIf()(spawn_timer >= 24):
            spawn_timer << 0
            DoActions(CreateUnit(2, "Zerg Zergling", "Anywhere", P8))
            adds_spawned += 2
        EUDEndIf()
    EUDEndIf()

    # 3. Damage intake: drain the hit register into boss_hp.
    if EUDIf()(boss_hits > applied_hits):
        boss_hp -= HIT_DAMAGE * (boss_hits - applied_hits)
        applied_hits << boss_hits
    EUDEndIf()

    # 4. Enrage: hand-rolled rising edge on the 25% threshold.
    if EUDIf()(boss_hp <= ENRAGE_AT):
        if EUDIf()(prev_enrage == 0):
            if EUDIf()(phase == 1):
                phase << 2
                DoActions(CreateUnit(4, "Zerg Hydralisk", "Anywhere", P8))
                adds_spawned += 4
            EUDEndIf()
        EUDEndIf()
        prev_enrage << 1
    EUDEndIf()
    if EUDIf()(boss_hp > ENRAGE_AT):
        prev_enrage << 0
    EUDEndIf()

    # 5. HP bar: rebuild on change, display every frame while up.
    if EUDIf()(phase >= 1):
        if EUDIf()(phase <= 2):
            if EUDIf()(boss_hp != shown_hp):
                shown_hp << boss_hp
                bar.insertf(0, "Boss HP: %D ", boss_hp)
                bar_fill << boss_hp // 50  # 0..20 blocks
                bar_k << 0
                if EUDWhile()(bar_k < bar_fill):
                    bar.append("#")
                    bar_k += 1
                EUDEndWhile()
            EUDEndIf()
            bar.Display()
        EUDEndIf()
    EUDEndIf()

    # 6. Victory: HP drained -> payout, phase 3.
    if EUDIf()(boss_hp == 0):
        if EUDIf()(phase == 1):
            phase << 3
            gold += 500
            DoActions(SetDeaths(ORE_EPD, Add, 500, 0))
            cleanup_timer << 0
        EUDEndIf()
        if EUDIf()(phase == 2):
            phase << 3
            gold += 500
            DoActions(SetDeaths(ORE_EPD, Add, 500, 0))
            cleanup_timer << 0
        EUDEndIf()
    EUDEndIf()

    # 7. Cleanup: 48 frames after victory, reset to idle.
    if EUDIf()(phase == 3):
        cleanup_timer += 1
        if EUDIf()(cleanup_timer >= 48):
            phase << 0
            boss_hits << 0
            applied_hits << 0
            adds_spawned << 0
            shown_hp << 0xFFFFFFFF
        EUDEndIf()
    EUDEndIf()
