# 61 - heal aura (legacy eudplib)
#
# Intent: a healing pylon at pixel (1024, 1024). Every 20 loops the
# aura pulses: every living Player 1 Marine within Chebyshev range
# 256 of the pylon (posX and posY each within 768..1280) regains
# 512 HP (StarCraft stores HP x256; a Marine's full HP is
# 40 x 256 = 10240), capped at full. The pulse scans the unit list
# once and books the results: units scanned, heals applied, total
# HP restored -- the cap case books only the actual deficit
# (10240 - hp), not the nominal 512.
#
# Mechanism: the scan walks Player 1's own unit list with
# EUDLoopPlayerUnit (no owner test needed in the body) and reads
# the fields off the unit EPD (unitID/posX/posY words, HP dword).
# Only hp, whose VALUE feeds the cap arithmetic, is read as a
# value; the filters are plain comparisons on the read values in
# the classic style. Chebyshev range (square) instead of
# Euclidean: two axis range tests, no squares, no sqrt -- the
# standard aura approximation.
#
# Counting form (pilot convention): run() builds the scenario's
# triggers once; benchmarks/pilot/count_legacy.py calls it inside a
# fresh PushTriggerScope and reports the GetTriggerCounter() delta.
# In production the body is the beforeTriggerExec hook; run() is
# that body verbatim. The EPD offsets are the legacy unit-layout
# constants (hp = uepd+2; owner = byte at uepd+19 sub 0; posX/posY
# = words at uepd+10 sub 0/2; unitID = word at uepd+25 sub 0).
from eudkit import *

P1 = 0
MARINE = 0          # unit id

AURA_X = 1024       # pylon position (pixels)
AURA_Y = 1024
AURA_RANGE = 256
X_MIN = AURA_X - AURA_RANGE  # 768
X_MAX = AURA_X + AURA_RANGE  # 1280
Y_MIN = AURA_Y - AURA_RANGE
Y_MAX = AURA_Y + AURA_RANGE
MARINE_MAX_HP = 10240  # 40 HP x 256
HEAL_AMOUNT = 512      # 2 HP x 256
AURA_PERIOD = 20       # loops between pulses

scanned = EUDVariable(0)        # units seen by the last pulse
total_scans = EUDVariable(0)    # lifetime pulses run
healed_last = EUDVariable(0)    # heals applied by the last pulse
heals_applied = EUDVariable(0)  # lifetime heals
hp_restored = EUDVariable(0)    # lifetime HP restored (x256)
deficit = EUDVariable(0)        # scratch: HP deficit in cap band
aura_tick = EUDVariable(0)      # hand-rolled pulse timer
hp = EUDVariable(0)             # scratch: current unit's HP


def run():
    global scanned, total_scans, healed_last, heals_applied
    global hp_restored, deficit, aura_tick, hp
    aura_tick += 1
    if EUDIf()(aura_tick >= AURA_PERIOD):
        aura_tick << 0
        scanned << 0
        healed_last << 0
        total_scans += 1
        for _uptr, uepd in EUDLoopPlayerUnit(P1):
            scanned += 1
            if EUDIf()(f_wread_epd(uepd + 25, 0) == MARINE):
                if EUDIf()(f_wread_epd(uepd + 10, 0) >= X_MIN):
                    if EUDIf()(f_wread_epd(uepd + 10, 0) <= X_MAX):
                        if EUDIf()(f_wread_epd(uepd + 10, 2) >= Y_MIN):
                            if EUDIf()(f_wread_epd(uepd + 10, 2) <= Y_MAX):
                                hp << f_dwread_epd(uepd + 2)
                                if EUDIf()(hp < MARINE_MAX_HP):
                                    # Below the cap band: a flat
                                    # +512 stays under the cap.
                                    if EUDIf()(hp <= MARINE_MAX_HP - HEAL_AMOUNT):
                                        f_dwwrite_epd(
                                            uepd + 2, hp + HEAL_AMOUNT)
                                        hp_restored += HEAL_AMOUNT
                                    EUDEndIf()
                                    # Cap band: exact deficit.
                                    if EUDIf()(hp > MARINE_MAX_HP - HEAL_AMOUNT):
                                        deficit << MARINE_MAX_HP - hp
                                        hp_restored += deficit
                                        f_dwwrite_epd(
                                            uepd + 2, MARINE_MAX_HP)
                                    EUDEndIf()
                                    healed_last += 1
                                    heals_applied += 1
                                EUDEndIf()
                            EUDEndIf()
                        EUDEndIf()
                    EUDEndIf()
                EUDEndIf()
            EUDEndIf()
    EUDEndIf()
