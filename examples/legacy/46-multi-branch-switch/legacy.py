"""LEGACY multi-branch switch dispatch (benchmark 46).

Same intent as ours.js: a defense map's phase machine. `phase`
selects one of eight behaviors per frame: idle, countdown, wave
spawning (two tiers), boss, intermission shopping, victory,
defeat, and a default "corrupt state" guard. Legacy expresses the
dispatch with EUDSwitch / EUDSwitchCase / EUDSwitchDefault blocks.
Victory/defeat are evaluated outside the dispatch so they preempt.
"""

from eudkit import *

phase = EUDVariable(0)
wave_timer = EUDVariable(0)
wave_number = EUDVariable(0)
spawn_left = EUDVariable(0)
gold = EUDVariable(0)
kills = EUDVariable(0)

PH_IDLE = 0
PH_COUNTDOWN = 1
PH_WAVE_LIGHT = 2
PH_WAVE_HEAVY = 3
PH_BOSS = 4
PH_SHOP = 5
PH_VICTORY = 6
PH_DEFEAT = 7


def run():
    global phase, wave_timer, wave_number, spawn_left, gold, kills
    EUDSwitch(phase)
    if EUDSwitchCase()(PH_IDLE):
        # Wait for the start signal: any marine death starts the countdown.
        if EUDIf()(Deaths(CurrentPlayer, AtLeast, 1, "Terran Marine")):
            phase << PH_COUNTDOWN
            wave_timer << 72
        EUDEndIf()

    if EUDSwitchCase()(PH_COUNTDOWN):
        wave_timer -= 1
        if EUDIf()(wave_timer == 0):
            wave_number += 1
            spawn_left << 10 + wave_number * 2
            phase << PH_WAVE_LIGHT
        EUDEndIf()

    if EUDSwitchCase()(PH_WAVE_LIGHT):
        if EUDIf()(spawn_left > 0):
            DoActions(CreateUnit(1, "Zerg Zergling", "Anywhere", P8))
            spawn_left -= 1
        EUDEndIf()
        if EUDIf()(spawn_left == 0):
            phase << PH_WAVE_HEAVY
            spawn_left << wave_number
        EUDEndIf()

    if EUDSwitchCase()(PH_WAVE_HEAVY):
        if EUDIf()(spawn_left > 0):
            DoActions(CreateUnit(1, "Zerg Hydralisk", "Anywhere", P8))
            spawn_left -= 1
        EUDEndIf()
        if EUDIf()(spawn_left == 0):
            if EUDIf()(wave_number >= 5):
                phase << PH_BOSS
            EUDEndIf()
            if EUDIf()(wave_number < 5):
                phase << PH_SHOP
            EUDEndIf()
        EUDEndIf()

    if EUDSwitchCase()(PH_BOSS):
        if EUDIf()(Deaths(P8, AtLeast, 1, "Zerg Ultralisk")):
            gold += 500
            phase << PH_SHOP
        EUDEndIf()

    if EUDSwitchCase()(PH_SHOP):
        wave_timer << 240
        phase << PH_COUNTDOWN

    if EUDSwitchCase()(PH_VICTORY):
        DoActions([DisplayText("Victory!"), Victory()])

    if EUDSwitchCase()(PH_DEFEAT):
        DoActions([DisplayText("Defeat..."), Defeat()])

    if EUDSwitchDefault()():
        # Corrupt phase value: fail safe back to idle.
        phase << PH_IDLE
    EUDEndSwitch()

    # Victory/defeat preempt the dispatch.
    if EUDIf()(kills >= 300):
        phase << PH_VICTORY
    EUDEndIf()
    if EUDIf()(Deaths(CurrentPlayer, AtLeast, 1, "Terran Command Center")):
        phase << PH_DEFEAT
    EUDEndIf()
