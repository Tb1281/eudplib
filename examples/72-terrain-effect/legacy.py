# 72 - terrain effects: lava damage / spring healing (legacy)
#
# Intent: two hazard zones act on every unit standing in them,
# once per 12-loop tick:
#   lava field  (x 512..1024, y 2048..2560): 512 raw damage
#               (2 HP in the engine's 1/256 fixed point);
#               a unit at or below that takes a killing write.
#   spring      (x 3072..3584, y 512..1024): heals 256 raw
#               (1 HP), but only units below 10240 raw (40 HP),
#               the documented stand-in cap for a Marine-grade
#               heal target (a full per-type max-HP table is a
#               data problem, not a logic difference).
# Positions come from the CUnit posX/posY word fields; ownership
# does not matter -- the terrain hurts and heals everyone.
# Bookkeeping counts lava hits, kills, heals and total damage.
#
# Counting form (pilot convention): run() builds the scenario's
# triggers once; benchmarks/pilot/count_legacy.py calls it inside
# a fresh PushTriggerScope and reports the GetTriggerCounter()
# delta. The read/write call sites ride the import-time engines
# (session-amortized, outside the delta) -- note.md states it.
from eudkit import *

TICK_PERIOD = 12
LAVA_DAMAGE = 512        # 2 HP in 1/256 fixed point
SPRING_HEAL = 256        # 1 HP
HEAL_CAP = 10240         # 40 HP: heal only below this
LAVA = (512, 1024, 2048, 2560)      # x0, x1, y0, y1
SPRING = (3072, 3584, 512, 1024)

# CUnit dword offsets: hp +0x08 (dword 2), pos +0x28 (dword 10;
# posX low word, posY high word).
HP_EPD_OFF = 2
POS_EPD_OFF = 10

tick = EUDVariable()
lava_hits = EUDVariable()
lava_kills = EUDVariable()
heals = EUDVariable()
total_damage = EUDVariable()
scratch_hp = EUDVariable()


def in_rect(x, y, rect):
    x0, x1, y0, y1 = rect
    return [x >= x0, x <= x1, y >= y0, y <= y1]


def apply_terrain():
    global lava_hits, lava_kills, heals, total_damage, scratch_hp
    for ptr, epd in EUDLoopUnit():
        x = f_wread_epd(epd + POS_EPD_OFF, 0)
        y = f_wread_epd(epd + POS_EPD_OFF, 1)

        # Lava field: damage, or a killing write at the floor.
        if EUDIf()(in_rect(x, y, LAVA)):
            scratch_hp << f_dwread_epd(epd + HP_EPD_OFF)
            if EUDIf()(scratch_hp > LAVA_DAMAGE):
                f_dwwrite_epd(epd + HP_EPD_OFF,
                              scratch_hp - LAVA_DAMAGE)
            if EUDElse()():
                f_dwwrite_epd(epd + HP_EPD_OFF, 0)
                lava_kills += 1
            EUDEndIf()
            lava_hits += 1
            total_damage += LAVA_DAMAGE
        EUDEndIf()

        # Spring: heal, capped at the stand-in ceiling.
        if EUDIf()(in_rect(x, y, SPRING)):
            scratch_hp << f_dwread_epd(epd + HP_EPD_OFF)
            if EUDIf()(scratch_hp < HEAL_CAP):
                f_dwwrite_epd(epd + HP_EPD_OFF,
                              scratch_hp + SPRING_HEAL)
                heals += 1
            EUDEndIf()
        EUDEndIf()


def run():
    global tick
    tick += 1
    if EUDIf()(tick >= TICK_PERIOD):
        tick << 0
        apply_terrain()
    EUDEndIf()
