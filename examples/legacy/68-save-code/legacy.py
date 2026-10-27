# 68 - save code encode + display (legacy eudplib)
#
# Intent: the classic RPG save code. The player's progress --
# hero level (0..19), stage (0..9), gold tier (0..7) and an
# 8-bit boss-flag mask -- is packed into one decimal code with a
# check digit appended:
#   body  = ((level * 10 + stage) * 10 + gold_tier) * 1000 + flags
#   check = (level*3 + stage*5 + gold_tier*7 + flags*11) mod 10
#   code  = body * 10 + check
# The code is recomputed and printed (f_dbstr_print into a
# reusable DBString, then Display) whenever progress changes, so
# the player can write it down and re-enter it next session. A
# small progression driver stands in for gameplay: every 120
# loops the stage advances, rolling over into a level-up, and
# every third stage sets the next boss flag.
#
# Counting form (pilot convention): run() builds the scenario's
# triggers once; benchmarks/pilot/count_legacy.py calls it inside
# a fresh PushTriggerScope and reports the GetTriggerCounter()
# delta. DBString.Display() allocates a string id from the loaded
# map, so LoadMap happens at module level (outside the delta),
# exactly like the decimal scoreboard scenario (C-30).
from eudkit import *
from eudkit.string.dbstr import DBString

LoadMap("tests/outputmap/basemap/basemap.scx")

code_buf = DBString(64)     # the one reusable save-code buffer

level = EUDVariable()
stage = EUDVariable()
gold_tier = EUDVariable()
boss_flags = EUDVariable()
next_flag_bit = EUDVariable()  # next boss bit to set: 1, 2, 4, ...
code_body = EUDVariable()   # scratch: packed progress
check = EUDVariable()       # scratch: check digit
save_code = EUDVariable()
progress_tick = EUDVariable()
prints = EUDVariable()
codes_shown = EUDVariable()  # distinct codes displayed so far


def recompute_and_print():
    global code_body, check, save_code, prints, codes_shown
    code_body << (level * 10 + stage) * 10 + gold_tier
    code_body << code_body * 1000 + boss_flags
    check << (level * 3 + stage * 5 + gold_tier * 7
              + boss_flags * 11) % 10
    save_code << code_body * 10 + check
    f_dbstr_print(code_buf, "SAVE CODE: ", save_code)
    code_buf.Display()
    prints += 1
    codes_shown += 1


def run():
    global progress_tick, stage, level, boss_flags, next_flag_bit
    # One-time init of the doubling bit (flag itself is the gate).
    if EUDIf()(next_flag_bit == 0):
        next_flag_bit << 1
    EUDEndIf()

    # Progression driver (stands in for gameplay events).
    progress_tick += 1
    if EUDIf()(progress_tick >= 120):
        progress_tick << 0
        stage += 1
        if EUDIf()(stage >= 10):
            stage << 0
            if EUDIf()(level < 20):
                level += 1
            EUDEndIf()
        EUDEndIf()
        # Every third stage fells the next boss: set its flag
        # bit (the bit position itself doubles per boss).
        if EUDIf()(stage % 3 == 0):
            if EUDIf()(stage >= 1):
                boss_flags |= next_flag_bit
                next_flag_bit << next_flag_bit * 2
            EUDEndIf()
        EUDEndIf()
        recompute_and_print()
    EUDEndIf()
