"""Scenario 22 (legacy) — rename eight unit types via the TBL table.

Intent: at game start (once), give the mod's core roster custom
display names across all three races (Marine -> "Space Marine",
Ghost -> "Shadow Operative", and so on).
Legacy idiom: f_settblf with HAND-COMPUTED TBL ids (unit code + 1,
per edac/116421), offset 0, an explicit "{}" format string, and an
explicit encoding argument. The writer machinery is built once
inside the scope and shared by the later calls.
"""

from eudkit import *  # noqa: F403

# TBL id = unit code + 1 (the mapmaker computes this by hand).
TBL_MARINE = 0 + 1  # Terran Marine
TBL_GHOST = 1 + 1  # Terran Ghost
TBL_FIREBAT = 32 + 1  # Terran Firebat
TBL_MEDIC = 34 + 1  # Terran Medic
TBL_ZERGLING = 37 + 1  # Zerg Zergling
TBL_HYDRALISK = 38 + 1  # Zerg Hydralisk
TBL_ZEALOT = 65 + 1  # Protoss Zealot
TBL_DRAGOON = 66 + 1  # Protoss Dragoon

renamed = EUDVariable()  # 1 once the names are written


def run():
    global renamed
    if EUDIf()(renamed == 0):
        f_settblf(TBL_MARINE, 0, "{}", "Space Marine", encoding="UTF-8")
        f_settblf(TBL_GHOST, 0, "{}", "Shadow Operative", encoding="UTF-8")
        f_settblf(TBL_FIREBAT, 0, "{}", "Pyro Specialist", encoding="UTF-8")
        f_settblf(TBL_MEDIC, 0, "{}", "Field Surgeon", encoding="UTF-8")
        f_settblf(TBL_ZERGLING, 0, "{}", "Voidling", encoding="UTF-8")
        f_settblf(TBL_HYDRALISK, 0, "{}", "Spine Beast", encoding="UTF-8")
        f_settblf(TBL_ZEALOT, 0, "{}", "Blade Adept", encoding="UTF-8")
        f_settblf(TBL_DRAGOON, 0, "{}", "War Strider", encoding="UTF-8")
        renamed << 1
    EUDEndIf()
