# 50 - boss fight composite (modern eudkit)
#
# Expresses ideal intent:
# 1. State machine combining add spawning, combat damage intake, enrage, HP HUD, and cleanup.
# 2. Modern StringBuffer for live boss HP display (ADR 0004).
# 3. Clean local session state with zero global singletons.
import eudkit


BOSS_HP_MAX = 1000
ENRAGE_AT = 250
HIT_DAMAGE = 25

MARINE = 0
ZERGLING = 37
HYDRALISK = 38
ULTRALISK = 39
ANYWHERE = 0

from eudkit import session

phase = session.var(0)          # 0 idle, 1 fight, 2 enraged, 3 victory cleanup
boss_hp = session.var(0)
boss_hits = session.var(0)
applied_hits = session.var(0)
gold = session.var(0)
adds_spawned = session.var(0)
spawn_timer = session.var(0)
shown_hp = session.var(-1)
cleanup_timer = session.var(0)

bar = session.string(96)

@session.fn(main_loop=True)
def game_loop() -> None:
    global phase, boss_hp, boss_hits, applied_hits, gold, adds_spawned
    global spawn_timer, shown_hp, cleanup_timer

    # 1. Start signal
    if eudkit.Deaths(eudkit.CurrentPlayer, eudkit.AtLeast, 1, MARINE):
        if phase == 0:
            phase = 1
            boss_hp = BOSS_HP_MAX
            eudkit.CreateUnit(1, ULTRALISK, ANYWHERE, eudkit.P8)

    # 2. Add spawner (every 24 frames during combat)
    if phase == 1 or phase == 2:
        spawn_timer += 1
        if spawn_timer >= 24:
            spawn_timer = 0
            eudkit.CreateUnit(2, ZERGLING, ANYWHERE, eudkit.P8)
            adds_spawned += 2

    # 3. Damage intake
    if boss_hits > applied_hits:
        boss_hp -= HIT_DAMAGE * (boss_hits - applied_hits)
        applied_hits = boss_hits

    # 4. Enrage at 25% HP
    if boss_hp <= ENRAGE_AT:
        with eudkit.once():
            if phase == 1:
                phase = 2
                eudkit.CreateUnit(4, HYDRALISK, ANYWHERE, eudkit.P8)
                adds_spawned += 4

    # 5. HP HUD update
    if phase == 1 or phase == 2:
        if boss_hp != shown_hp:
            shown_hp = boss_hp
            bar.printf("Boss HP: %d", boss_hp)
        bar.display_text()

    # 6. Victory
    if boss_hp <= 0:
        if phase == 1 or phase == 2:
            phase = 3
            gold += 500
            eudkit.SetResources(eudkit.P1, eudkit.Add, 500, eudkit.Ore)
            cleanup_timer = 0

    # 7. Cleanup
    if phase == 3:
        cleanup_timer += 1
        if cleanup_timer >= 48:
            phase = 0
            boss_hits = 0
            applied_hits = 0
            adds_spawned = 0
            shown_hp = -1
