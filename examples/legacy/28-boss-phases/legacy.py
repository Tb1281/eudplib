"""LEGACY boss phase state machine (benchmark 28).

Same intent as ours.js: a boss with 3000 HP. While Player 1 keeps
at least 4 Marines in the boss zone (location 4), the boss takes
10 damage per trigger loop. Phases derive from HP: phase 1 above
2000, phase 2 at 2000..1001 (spawns 2 Zerglings at location 5
every 100 loops), phase 3 at 1000..1 (spawns 2 Zerglings + 1
Hydralisk every 60 loops, with a ping), phase 4 = dead (+1000 ore
reward, victory latch). Phase entries fire a one-time spawn burst
and reset the phase timer. Legacy state is explicit variables and
an if-chain; phase 0 is the uninitialized state (legacy variables
are zero-initialized, so HP/phase are set by a first-loop block).
"""

from eudkit import *

BOSS_MAX_HP = 3000
DAMAGE_PER_LOOP = 10
MARINES_FOR_DAMAGE = 4
LOC_BOSS_ZONE = 4        # location ids (numeric: no basemap lookup)
LOC_SPAWN = 5
UNIT_MARINE = 0          # unit ids (numeric form, as in the pilot)
UNIT_ZERGLING = 37
UNIT_HYDRALISK = 34
P2_PERIOD = 100          # loops between phase-2 spawns
P3_PERIOD = 60           # loops between phase-3 spawns
DEATH_REWARD_ORE = 1000

boss_hp = EUDVariable()      # remaining boss HP
phase = EUDVariable()        # 0 = uninit, 1/2/3 = alive, 4 = dead
new_phase = EUDVariable()    # phase implied by current HP (scratch)
phase_changed = EUDVariable()  # scratch flag for the transition test
phase_timer = EUDVariable()  # loops in the current phase (spawn timer)
minion_spawns = EUDVariable()  # lifetime minions spawned by phases
victory = EUDVariable()      # latched on boss death


def run():
    global boss_hp, phase, new_phase, phase_changed, phase_timer
    global minion_spawns, victory
    # One-time init: legacy variables start at 0.
    if EUDIf()(phase == 0):
        boss_hp << BOSS_MAX_HP
        phase << 1
    EUDEndIf()

    if EUDIf()(victory == 0):
        # Damage: marines in the zone grind the boss down.
        if EUDIf()([Bring(Player1, AtLeast, MARINES_FOR_DAMAGE,
                          UNIT_MARINE, LOC_BOSS_ZONE), boss_hp >= 1]):
            boss_hp -= DAMAGE_PER_LOOP
        EUDEndIf()

        # Phase from HP: later checks refine earlier ones.
        new_phase << phase
        if EUDIf()(boss_hp <= 2000):
            new_phase << 2
        EUDEndIf()
        if EUDIf()(boss_hp <= 1000):
            new_phase << 3
        EUDEndIf()
        if EUDIf()(boss_hp == 0):
            new_phase << 4
        EUDEndIf()

        # Transition test without a != compare: two ordering checks.
        phase_changed << 0
        if EUDIf()(new_phase > phase):
            phase_changed << 1
        EUDEndIf()
        if EUDIf()(new_phase < phase):
            phase_changed << 1
        EUDEndIf()

        # Transition effects (one-time per phase entry).
        if EUDIf()(phase_changed == 1):
            if EUDIf()(new_phase == 2):
                DoActions(CreateUnit(4, UNIT_HYDRALISK, LOC_SPAWN, Player8))
                DoActions(MinimapPing(LOC_SPAWN))
            EUDEndIf()
            if EUDIf()(new_phase == 3):
                DoActions(CreateUnit(6, UNIT_ZERGLING, LOC_SPAWN, Player8))
                DoActions(MinimapPing(LOC_SPAWN))
            EUDEndIf()
            if EUDIf()(new_phase == 4):
                DoActions(SetResources(Player1, Add, DEATH_REWARD_ORE, Ore))
                DoActions(MinimapPing(LOC_BOSS_ZONE))
                victory << 1
            EUDEndIf()
            phase << new_phase
            phase_timer << 0
        EUDEndIf()

        # Phase behavior: periodic minion spawns.
        if EUDIf()(phase == 2):
            phase_timer += 1
            if EUDIf()(phase_timer >= P2_PERIOD):
                phase_timer << 0
                DoActions(CreateUnit(2, UNIT_ZERGLING, LOC_SPAWN, Player8))
                minion_spawns += 2
            EUDEndIf()
        EUDEndIf()
        if EUDIf()(phase == 3):
            phase_timer += 1
            if EUDIf()(phase_timer >= P3_PERIOD):
                phase_timer << 0
                DoActions([
                    CreateUnit(2, UNIT_ZERGLING, LOC_SPAWN, Player8),
                    CreateUnit(1, UNIT_HYDRALISK, LOC_SPAWN, Player8),
                    MinimapPing(LOC_SPAWN),
                ])
                minion_spawns += 3
            EUDEndIf()
        EUDEndIf()
    EUDEndIf()
