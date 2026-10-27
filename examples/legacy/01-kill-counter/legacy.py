"""LEGACY kill counter (pilot 1).

Same intent as ours_1_kill_counter.js: an escalating-threshold kill
reward. Every trigger loop the engine's Marine deaths counter for
Player1 is read into a variable (f_dwread_epd) and compared against a
variable threshold (var-vs-var compare). When reached: pay 100 ore via
SetResources, consume the counter via SetDeaths (reset to 0), credit
the consumed kills, and escalate the next threshold by 15.
"""

from eudkit import *

# Deaths table address for (Player1, Terran Marine) =
# 0x58A364 + 4*player + 48*unit, player 0, unit 0.
MARINE_DEATHS_P1 = 0x58A364

# Engine-facing state (per-frame loop variables).
cur_kills = EUDVariable()        # snapshot of the engine counter
kills_credited = EUDVariable()   # total kills rewarded so far
next_reward = EUDVariable()      # escalating threshold: 10, 25, 40, ... (set on frame 1)
rewards_paid = EUDVariable()     # payout bookkeeping
rewards_this_wave = EUDVariable()  # resets every 3 payouts
ore_paid_total = EUDVariable()   # lifetime ore granted
best_wave = EUDVariable()        # most payouts in any single wave


def on_first_frame():
    next_reward << 10
    rewards_this_wave << 0


def run():
    global cur_kills, kills_credited, next_reward, rewards_paid, rewards_this_wave
    global ore_paid_total, best_wave
    # One-time init guard (legacy has no Once gate idiom here, so the
    # flag itself is the gate).
    if EUDIf()(rewards_paid == 0):
        on_first_frame()
    EUDEndIf()

    # Read the counter (materialize into a var: legacy's standard form
    # for doing anything beyond a constant comparison).
    cur_kills << f_dwread_epd(EPD(MARINE_DEATHS_P1))

    # Reward gate: var-vs-var compare (the threshold is dynamic).
    if EUDIf()(cur_kills >= next_reward):
        # Pay out: +100 ore for Player1, as a direct memory write
        # (SetDeaths addressed at the ore dword via its EPD).
        DoActions(SetDeaths(EPD(0x57F0F0), Add, 100, 0))
        ore_paid_total += 100
        # Consume: reset the engine counter to 0 via SetDeaths.
        DoActions(SetDeaths(Player1, SetTo, 0, 0))  # 0 = Terran Marine (id form: no basemap name lookup needed)
        # Bookkeeping: credit exactly the threshold that was consumed.
        kills_credited += next_reward
        next_reward += 15
        rewards_paid += 1
        rewards_this_wave += 1
        # Wave bonus: every 3rd payout grants an extra 50 ore,
        # records the wave in the best-wave tracker, and re-arms.
        if EUDIf()(rewards_this_wave >= 3):
            DoActions(SetDeaths(EPD(0x57F0F0), Add, 50, 0))
            ore_paid_total += 50
            if EUDIf()(rewards_this_wave > best_wave):
                best_wave << rewards_this_wave
            EUDEndIf()
            rewards_this_wave << 0
        EUDEndIf()
    EUDEndIf()
