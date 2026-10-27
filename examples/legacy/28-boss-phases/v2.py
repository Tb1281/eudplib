"""Scenario 28: Boss Phase State Machine — Modern eudkit (v2).

Ideal Intent:
Autonomous multi-phase boss fight with 3000 initial HP:
1. When Player 1 keeps at least 4 Marines in the boss zone (location 4),
   the boss takes 10 damage per loop.
2. Phase 1 (>2000 HP): standard combat phase.
3. Phase 2 (<=2000 HP):
   - Entry burst: 4 Hydralisks spawned at location 5, ping spawn zone.
   - Periodic: spawns 2 Zerglings every 100 loops.
4. Phase 3 (<=1000 HP):
   - Entry burst: 6 Zerglings spawned at location 5, ping spawn zone.
   - Periodic: spawns 2 Zerglings + 1 Hydralisk every 60 loops, ping spawn zone.
5. Phase 4 (==0 HP):
   - Victory reward: 1000 Ore to Player 1, ping boss zone, latches victory.
Modern v2 uses direct variable initialization and clean state transitions,
eliminating legacy zero-init workarounds and scratch transition flags.
"""

from pathlib import Path
import eudkit
from eudkit import (
    Add,
    AtLeast,
    Bring,
    CreateUnit,
    MinimapPing,
    Ore,
    Player1,
    Player8,
    SetResources,
)

REPO_ROOT = Path(__file__).resolve().parents[3]
BASEMAP_PATH = REPO_ROOT / "tests" / "outputmap" / "basemap" / "basemap_strx.scx"

BOSS_MAX_HP = 3000
DAMAGE_PER_LOOP = 10
MARINES_FOR_DAMAGE = 4
LOC_BOSS_ZONE = 4
LOC_SPAWN = 5
P2_PERIOD = 100
P3_PERIOD = 60
DEATH_REWARD_ORE = 1000


def build(session: eudkit.Session):
    boss_hp = session.var(BOSS_MAX_HP, name="boss_hp")
    phase = session.var(1, name="phase")
    phase_timer = session.var(0, name="phase_timer")
    minion_spawns = session.var(0, name="minion_spawns")
    victory = session.var(0, name="victory")

    @session.fn
    def game_loop():
        if victory == 0:
            # Boss damage: Marines in zone deal 10 damage per loop
            if Bring(Player1, AtLeast, MARINES_FOR_DAMAGE, "Terran Marine", LOC_BOSS_ZONE):
                if boss_hp >= 1:
                    boss_hp -= DAMAGE_PER_LOOP

            # Phase 2 transition (at or below 2000 HP)
            if (phase == 1) and (boss_hp <= 2000):
                phase = 2
                phase_timer = 0
                CreateUnit(4, "Zerg Hydralisk", LOC_SPAWN, Player8)
                MinimapPing(LOC_SPAWN)

            # Phase 3 transition (at or below 1000 HP)
            if (phase == 2) and (boss_hp <= 1000):
                phase = 3
                phase_timer = 0
                CreateUnit(6, "Zerg Zergling", LOC_SPAWN, Player8)
                MinimapPing(LOC_SPAWN)

            # Phase 4 victory transition (boss HP reaches 0)
            if (phase == 3) and (boss_hp == 0):
                phase = 4
                SetResources(Player1, Add, DEATH_REWARD_ORE, Ore)
                MinimapPing(LOC_BOSS_ZONE)
                victory = 1

            # Phase 2 periodic minion spawn (every 100 loops)
            if phase == 2:
                phase_timer += 1
                if phase_timer >= P2_PERIOD:
                    phase_timer = 0
                    CreateUnit(2, "Zerg Zergling", LOC_SPAWN, Player8)
                    minion_spawns += 2

            # Phase 3 periodic minion spawn (every 60 loops)
            if phase == 3:
                phase_timer += 1
                if phase_timer >= P3_PERIOD:
                    phase_timer = 0
                    CreateUnit(2, "Zerg Zergling", LOC_SPAWN, Player8)
                    CreateUnit(1, "Zerg Hydralisk", LOC_SPAWN, Player8)
                    MinimapPing(LOC_SPAWN)
                    minion_spawns += 3

    return {
        "boss_hp": boss_hp,
        "phase": phase,
        "minion_spawns": minion_spawns,
        "victory": victory,
    }


def main():
    session = eudkit.Session(BASEMAP_PATH)
    build(session)
    out_path = Path(__file__).parent / "output_v2.scx"
    report = session.compile(out_path)
    print("Scenario 28 (v2) compiled successfully:")
    print(f"  Triggers: {report.trigger_count}")
    print(f"  Payload:  {report.payload_size} bytes")
    print(f"  VProcs:   {report.vproc_count}")
    return report


if __name__ == "__main__":
    main()
