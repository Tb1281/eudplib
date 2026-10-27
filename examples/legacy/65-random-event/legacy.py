# 65 - probability event wheel (legacy eudplib)
#
# Intent: every 480 trigger loops the map rolls one global event
# (uniform 0..99 from the game RNG):
#   roll < 15   windfall:   every player gains 200 ore
#   15..29      gas rush:   every player gains 100 gas
#   30..44      tax day:    every player loses 50 gas (the engine's
#                           SetResources Subtract clamps at zero,
#                           so nobody can go negative)
#   45..59      critters:   4 Rhynadons wander in at the center
#   60..99      quiet:      nothing happens
# Bookkeeping keeps a counter per outcome, the last event code, and
# the total ore the windfall has paid over the game's lifetime.
#
# Counting form (pilot convention): run() builds the scenario's
# triggers once; benchmarks/pilot/count_legacy.py calls it inside a
# fresh PushTriggerScope and reports the GetTriggerCounter() delta.
# Units, locations and players are numeric ids (no basemap); text
# is a Db string, as in the pilot scenarios. The f_dwrand call site
# rides legacy's EUDFunc machinery, whose body is created at its
# first call in the session (see note.md on the counting boundary).
from eudkit import *

RHYNADON = 89
CENTER = 40
NEUTRAL = 11
PERIOD = 480
EVENT_COUNT = 4          # human players receiving grants/taxes

event_timer = EUDVariable()
roll = EUDVariable()               # scratch: the current draw
last_event = EUDVariable()         # 0 none yet, 1..5 outcome codes
windfall_events = EUDVariable()
gas_events = EUDVariable()
tax_events = EUDVariable()
critter_events = EUDVariable()
quiet_events = EUDVariable()
total_windfall_ore = EUDVariable()


def run():
    global event_timer, roll, last_event, total_windfall_ore
    global windfall_events, gas_events, tax_events, critter_events
    global quiet_events
    event_timer += 1
    if EUDIf()(event_timer >= PERIOD):
        event_timer << 0
        roll << f_dwrand() % 100

        # Outcome chain (mutually exclusive ranges of the roll).
        if EUDIf()(roll < 15):
            for p in range(EVENT_COUNT):
                DoActions(SetResources(EncodePlayer(p), Add, 200, Ore))
            total_windfall_ore += 200 * EVENT_COUNT
            windfall_events += 1
            last_event << 1
            DoActions(DisplayText(Db("Random event: mineral windfall!")))
        if EUDElseIf()(roll < 30):
            for p in range(EVENT_COUNT):
                DoActions(SetResources(EncodePlayer(p), Add, 100, Gas))
            gas_events += 1
            last_event << 2
            DoActions(DisplayText(Db("Random event: gas rush!")))
        if EUDElseIf()(roll < 45):
            for p in range(EVENT_COUNT):
                DoActions(SetResources(EncodePlayer(p), Subtract, 50, Gas))
            tax_events += 1
            last_event << 3
            DoActions(DisplayText(Db("Random event: tax day.")))
        if EUDElseIf()(roll < 60):
            DoActions(CreateUnit(4, RHYNADON, CENTER,
                                 EncodePlayer(NEUTRAL)))
            critter_events += 1
            last_event << 4
            DoActions(DisplayText(Db("Random event: critters wander in.")))
        if EUDElse()():
            quiet_events += 1
            last_event << 5
        EUDEndIf()
    EUDEndIf()
