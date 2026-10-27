# 55 - quest tracker (legacy eudplib)
#
# Intent: a three-quest campaign chain for Player 1, tracked
# through Player 8's Deaths table:
#   Q1: kill 10 Zerglings  -> +150 ore
#   Q2: kill 5 Hydralisks  -> +250 ore, +50 gas
#   Q3: kill 2 Ultralisks  -> +500 ore, campaign complete latch
# All three counters are read every loop and their prev-values
# updated unconditionally, so switching quests never produces a
# phantom delta; only the active quest's target type feeds the
# progress var. Every 48 loops a quest log line is refreshed
# ("QUEST 2: 3/5"). Completion pays out, resets progress, and
# advances the quest var in the same loop.
#
# Counting form (pilot convention): run() builds the scenario's
# triggers once; benchmarks/pilot/count_legacy.py calls it inside a
# fresh PushTriggerScope and reports the GetTriggerCounter() delta.
# In production the body is the beforeTriggerExec hook; run() is
# that body verbatim. Units are numeric ids (no basemap in the
# harness). LoadMap is module level: DBString.Display() allocates
# a string id from the loaded map, as in any real legacy build.
from eudkit import *
from eudkit.string.dbstr import DBString

LoadMap("tests/outputmap/basemap/basemap.scx")

P1 = 0
P8 = 7
LOC_BASE = 1        # location id

ZERGLING = 37       # unit ids
HYDRALISK = 38
ULTRALISK = 39

# Quest table: (target unit, target count, ore reward, gas reward).
QUESTS = [
    (ZERGLING, 10, 150, 0),
    (HYDRALISK, 5, 250, 50),
    (ULTRALISK, 2, 500, 0),
]
LOG_PERIOD = 48     # loops between quest-log refreshes

# Deaths table addresses for Player 8's units (see scenario 53).
DEATHS_ADDRS = [
    0x58A364 + 4 * P8 + 48 * ZERGLING,
    0x58A364 + 4 * P8 + 48 * HYDRALISK,
    0x58A364 + 4 * P8 + 48 * ULTRALISK,
]

quest = EUDVariable(1)          # current quest 1..3 (4 = all done)
progress = EUDVariable(0)       # kills toward the active target
campaign_done = EUDVariable(0)  # latched after Q3
completions = EUDVariable(0)    # quests finished
total_kills = EUDVariable(0)    # kills credited to quests
prev = [EUDVariable(0) for _ in range(3)]  # counters at last loop
cur = [EUDVariable(0) for _ in range(3)]   # scratch: current counters
log_tick = EUDVariable(0)       # hand-rolled refresh timer

quest_log = DBString(96)        # the one reusable log buffer


def run():
    global quest, progress, campaign_done, completions, total_kills
    global log_tick
    if EUDIf()(campaign_done == 0):
        # Track kills: read all three counters, credit only the
        # active quest's type, update every prev unconditionally.
        for i in range(3):
            cur[i] << f_dwread_epd(EPD(DEATHS_ADDRS[i]))
            if EUDIf()(quest == i + 1):
                if EUDIf()(cur[i] > prev[i]):
                    progress += cur[i] - prev[i]
                    total_kills += cur[i] - prev[i]
                EUDEndIf()
            EUDEndIf()
            prev[i] << cur[i]

        # Completion: one branch per quest, constant target.
        for i, (_unit, target, ore, gas) in enumerate(QUESTS):
            if EUDIf()(quest == i + 1):
                if EUDIf()(progress >= target):
                    DoActions(SetResources(P1, Add, ore, Ore))
                    if gas > 0:
                        DoActions(SetResources(P1, Add, gas, Gas))
                    DoActions(MinimapPing(LOC_BASE))
                    progress << 0
                    quest += 1
                    completions += 1
                    if EUDIf()(quest == 4):
                        campaign_done << 1
                    EUDEndIf()
                EUDEndIf()
            EUDEndIf()
    EUDEndIf()

    # Quest log refresh (hand-rolled timer). f_dbstr_print composes
    # the whole line per call, so each quest branch prints its own
    # complete line with its constant target.
    log_tick += 1
    if EUDIf()(log_tick >= LOG_PERIOD):
        log_tick << 0
        for i, (_unit, target, _ore, _gas) in enumerate(QUESTS):
            if EUDIf()(quest == i + 1):
                f_dbstr_print(quest_log, "QUEST ", quest, ": ",
                              progress, "/", target)
            EUDEndIf()
        if EUDIf()(quest == 4):
            f_dbstr_print(quest_log, "QUEST 4: DONE")
        EUDEndIf()
        quest_log.Display()
    EUDEndIf()
