# 67 - boss nova countdown (legacy eudplib)
#
# Intent: the boss fires a nova on a fixed 600-loop cycle. The
# blast is telegraphed: at T-72, T-48 and T-24 loops a warning
# sound plays and a countdown number is displayed ("3", "2",
# "1"), so players can pull their army out of the arena. At T-0
# every Player 1 unit standing inside the arena rectangle
# (x 1024..3072, y 1024..3072, read from the CUnit position
# fields) takes 25600 raw damage -- 100 HP in the engine's
# 1/256 fixed point; units at or below that threshold die.
# Bookkeeping counts novas, warnings, and units caught per blast.
#
# Counting form (pilot convention): run() builds the scenario's
# triggers once; benchmarks/pilot/count_legacy.py calls it inside
# a fresh PushTriggerScope and reports the GetTriggerCounter()
# delta. Units, locations and players are numeric ids (no
# basemap); text and sound are Db / string ids, as in the pilot.
from eudkit import *

# PlayWAV encodes its sound name against the map string table, so
# the harness loads the basemap at module level -- outside the
# counted run() delta, exactly like the DBString scenarios do.
LoadMap("tests/outputmap/basemap/basemap.scx")

P1 = 0
CYCLE = 600              # loops between novas
WARN_STEP = 24           # loops between countdown warnings
NOVA_DAMAGE = 25600      # 100 HP in 1/256 fixed point
ARENA_X0, ARENA_X1 = 1024, 3072
ARENA_Y0, ARENA_Y1 = 1024, 3072

# CUnit dword offsets (field offset / 4): hp +0x08, pos +0x28
# (posX is the low word, posY the high word of dword 10),
# owner +0x4C (byte 0 of dword 19).
HP_EPD_OFF = 2
POS_EPD_OFF = 10
OWNER_EPD_OFF = 19

cycle_t = EUDVariable()          # loops into the current cycle
warn_stage = EUDVariable()       # next warning to fire: 3, 2, 1, 0
nova_count = EUDVariable()
warnings_shown = EUDVariable()
last_blast_hits = EUDVariable()  # units hit by the latest nova
total_hits = EUDVariable()
scratch_hp = EUDVariable()


def fire_warning(stage):
    DoActions(PlayWAV("sound\\glue\\bnetclick.wav"))
    if stage == 3:
        DoActions(DisplayText(Db("Boss nova in 3...")))
    elif stage == 2:
        DoActions(DisplayText(Db("Boss nova in 2...")))
    else:
        DoActions(DisplayText(Db("Boss nova in 1...")))


def nova_blast():
    global last_blast_hits, total_hits, scratch_hp
    last_blast_hits << 0
    for ptr, epd in EUDLoopUnit():
        # Victims: Player 1's units inside the arena rectangle.
        if EUDIf()(f_bread_epd(epd + OWNER_EPD_OFF, 0) == P1):
            x = f_wread_epd(epd + POS_EPD_OFF, 0)
            y = f_wread_epd(epd + POS_EPD_OFF, 1)
            if EUDIf()([x >= ARENA_X0, x <= ARENA_X1,
                        y >= ARENA_Y0, y <= ARENA_Y1]):
                scratch_hp << f_dwread_epd(epd + HP_EPD_OFF)
                if EUDIf()(scratch_hp > NOVA_DAMAGE):
                    f_dwwrite_epd(epd + HP_EPD_OFF,
                                  scratch_hp - NOVA_DAMAGE)
                if EUDElse()():
                    f_dwwrite_epd(epd + HP_EPD_OFF, 0)
                EUDEndIf()
                last_blast_hits += 1
                total_hits += 1
            EUDEndIf()
        EUDEndIf()


def run():
    global cycle_t, warn_stage, nova_count, warnings_shown
    cycle_t += 1

    # Warning schedule: stages 3/2/1 at CYCLE-72/-48/-24.
    if EUDIf()([warn_stage == 3, cycle_t >= CYCLE - 3 * WARN_STEP]):
        fire_warning(3)
        warn_stage << 2
        warnings_shown += 1
    EUDEndIf()
    if EUDIf()([warn_stage == 2, cycle_t >= CYCLE - 2 * WARN_STEP]):
        fire_warning(2)
        warn_stage << 1
        warnings_shown += 1
    EUDEndIf()
    if EUDIf()([warn_stage == 1, cycle_t >= CYCLE - WARN_STEP]):
        fire_warning(1)
        warn_stage << 0
        warnings_shown += 1
    EUDEndIf()

    # Detonation and cycle reset.
    if EUDIf()(cycle_t >= CYCLE):
        nova_blast()
        nova_count += 1
        cycle_t << 0
        warn_stage << 3
    EUDEndIf()
