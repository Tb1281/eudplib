"""Scenario 23 (legacy) — rename six ranks + the attack button text.

Intent: at game start (once), relabel the rank ladder the mod uses
(Recruit -> "Rookie", Corporal -> "Veteran", Sergeant -> "Drill
Sergeant", Master Sergeant -> "First Sergeant", Captain ->
"Commander", Colonel -> "Wing Commander", Admiral -> "Fleet
Admiral") and set the attack button's text to "Charge!".
Legacy idiom: f_settblf with hand-looked-up TBL ids from the rank
block (1302..1324) and the button-set entry 1400 — whose text starts
at offset 2, after the "<key><01>" prefix the mapmaker must know to
skip by hand.
"""

from eudkit import *  # noqa: F403

TBL_RANK_RECRUIT = 1302
TBL_RANK_CORPORAL = 1306
TBL_RANK_SERGEANT = 1308
TBL_RANK_MASTER_SERGEANT = 1310
TBL_RANK_CAPTAIN = 1312
TBL_RANK_COLONEL = 1316
TBL_RANK_ADMIRAL = 1324
TBL_BUTTON_SET = 1400
BUTTON_TEXT_OFFSET = 2  # skip the button-set "<Key Q><01>" prefix

relabeled = EUDVariable()  # 1 once the labels are written


def run():
    global relabeled
    if EUDIf()(relabeled == 0):
        f_settblf(TBL_RANK_RECRUIT, 0, "{}", "Rookie", encoding="UTF-8")
        f_settblf(TBL_RANK_CORPORAL, 0, "{}", "Veteran", encoding="UTF-8")
        f_settblf(TBL_RANK_SERGEANT, 0, "{}", "Drill Sergeant", encoding="UTF-8")
        f_settblf(TBL_RANK_MASTER_SERGEANT, 0, "{}", "First Sergeant", encoding="UTF-8")
        f_settblf(TBL_RANK_CAPTAIN, 0, "{}", "Commander", encoding="UTF-8")
        f_settblf(TBL_RANK_COLONEL, 0, "{}", "Wing Commander", encoding="UTF-8")
        f_settblf(TBL_RANK_ADMIRAL, 0, "{}", "Fleet Admiral", encoding="UTF-8")
        f_settblf(TBL_BUTTON_SET, BUTTON_TEXT_OFFSET, "{}", "Charge!", encoding="UTF-8")
        relabeled << 1
    EUDEndIf()
