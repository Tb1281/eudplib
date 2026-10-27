"""LEGACY switch toggle puzzle (benchmark 33).

Same intent as ours.js: a door guarded by 4 switches (ids 0..3,
toggled by classic triggers outside this code). The door opens
when the combination is exactly: switch 0 Set, switch 1 Cleared,
switch 2 Set, switch 3 Set. Every trigger loop the combination is
re-scored (match_count = how many switches match their target).
On the loop where the score reaches 4 (a solved EDGE -- the latch
prevents re-firing while the combination holds), Player 1 gets
300 ore and the door location is pinged. When exactly 3 of 4
match, a throttled hint ping fires (at most once per 240 loops).
Bookkeeping: solves, near-miss loops, hints shown.
"""

from eudkit import *

SW_A, SW_B, SW_C, SW_D = 0, 1, 2, 3   # switch ids
LOC_DOOR = 7             # location id (numeric: no basemap lookup)
SOLVE_ORE = 300
HINT_COOLDOWN = 240      # loops between hint pings

match_count = EUDVariable()  # switches matching the target combo
solved_latch = EUDVariable()  # 1 while the combo holds (edge guard)
solves = EUDVariable()
near_miss_loops = EUDVariable()  # loops with exactly 3 of 4 matching
hints = EUDVariable()
hint_cd = EUDVariable()      # loops until the next hint may fire


def run():
    global match_count, solved_latch, solves, near_miss_loops, hints
    global hint_cd
    # Hint throttle ticks every loop.
    if EUDIf()(hint_cd >= 1):
        hint_cd -= 1
    EUDEndIf()

    # Re-score the combination from the four switch states.
    match_count << 0
    if EUDIf()(Switch(SW_A, Set)):
        match_count += 1
    EUDEndIf()
    if EUDIf()(Switch(SW_B, Cleared)):
        match_count += 1
    EUDEndIf()
    if EUDIf()(Switch(SW_C, Set)):
        match_count += 1
    EUDEndIf()
    if EUDIf()(Switch(SW_D, Set)):
        match_count += 1
    EUDEndIf()

    # Solved edge: fire once when the combo completes, re-arm when
    # it breaks. The latch is the hand-rolled rising-edge detector.
    if EUDIf()([match_count >= 4, solved_latch == 0]):
        DoActions([
            SetResources(Player1, Add, SOLVE_ORE, Ore),
            MinimapPing(LOC_DOOR),
        ])
        solves += 1
        solved_latch << 1
    EUDEndIf()
    if EUDIf()([match_count <= 3, solved_latch == 1]):
        solved_latch << 0
    EUDEndIf()

    # Near-miss hint, throttled.
    if EUDIf()(match_count == 3):
        near_miss_loops += 1
        if EUDIf()(hint_cd == 0):
            DoActions(MinimapPing(LOC_DOOR))
            hints += 1
            hint_cd << HINT_COOLDOWN
        EUDEndIf()
    EUDEndIf()
