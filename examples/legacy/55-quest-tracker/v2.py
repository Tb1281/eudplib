# 55 - quest tracker (modern eudkit)
#
# Expresses ideal intent:
# 1. Campaign progression tracking through Player 8 Deaths entries.
# 2. Modern `StringBuffer` for quest-log HUD updates (ADR 0004).
# 3. Reactive `with every(48):` clock eliminates manual log tick counter.
import eudkit


P1 = 0
P8 = 7
LOC_BASE = 1

ZERGLING = 37
HYDRALISK = 38
ULTRALISK = 39

LOG_PERIOD = 48

DEATHS_ZERGLING = 0x58A364 + 4 * P8 + 48 * ZERGLING
DEATHS_HYDRALISK = 0x58A364 + 4 * P8 + 48 * HYDRALISK
DEATHS_ULTRALISK = 0x58A364 + 4 * P8 + 48 * ULTRALISK

from eudkit import session

quest = session.var(1)
progress = session.var(0)
campaign_done = session.var(0)
completions = session.var(0)
total_kills = session.var(0)
prev_z = session.var(0)
prev_h = session.var(0)
prev_u = session.var(0)

quest_log = session.string(96)

@session.fn(main_loop=True)
def game_loop() -> None:
    global quest, progress, campaign_done, completions, total_kills
    global prev_z, prev_h, prev_u

    if campaign_done == 0:
        cur_z = eudkit.dwread(DEATHS_ZERGLING)
        cur_h = eudkit.dwread(DEATHS_HYDRALISK)
        cur_u = eudkit.dwread(DEATHS_ULTRALISK)

        if quest == 1:
            if cur_z > prev_z:
                progress += cur_z - prev_z
                total_kills += cur_z - prev_z
        elif quest == 2:
            if cur_h > prev_h:
                progress += cur_h - prev_h
                total_kills += cur_h - prev_h
        elif quest == 3:
            if cur_u > prev_u:
                progress += cur_u - prev_u
                total_kills += cur_u - prev_u

        prev_z = cur_z
        prev_h = cur_h
        prev_u = cur_u

        # Q1 completion
        if quest == 1:
            if progress >= 10:
                eudkit.SetResources(P1, eudkit.Add, 150, eudkit.Ore)
                eudkit.MinimapPing(LOC_BASE)
                progress = 0
                quest = 2
                completions += 1

        # Q2 completion
        if quest == 2:
            if progress >= 5:
                eudkit.SetResources(P1, eudkit.Add, 250, eudkit.Ore)
                eudkit.SetResources(P1, eudkit.Add, 50, eudkit.Gas)
                eudkit.MinimapPing(LOC_BASE)
                progress = 0
                quest = 3
                completions += 1

        # Q3 completion
        if quest == 3:
            if progress >= 2:
                eudkit.SetResources(P1, eudkit.Add, 500, eudkit.Ore)
                eudkit.MinimapPing(LOC_BASE)
                progress = 0
                quest = 4
                completions += 1
                campaign_done = 1

    with eudkit.every(LOG_PERIOD):
        if quest == 1:
            quest_log.printf("QUEST 1: %d/10", progress)
        elif quest == 2:
            quest_log.printf("QUEST 2: %d/5", progress)
        elif quest == 3:
            quest_log.printf("QUEST 3: %d/2", progress)
        elif quest == 4:
            quest_log.printf("QUEST 4: DONE")
        quest_log.display_text()
