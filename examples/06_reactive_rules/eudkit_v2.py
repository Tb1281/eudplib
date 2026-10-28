"""Example 06: Reactive Rules & Events — Modern eudkit (v2).

Demonstrates:
- Declarative reactive primitives:
  - with once(): / once(() => ...): Executes once on start or when condition matches
  - with every(ticks): / every(ticks, () => ...): Repeats cyclically every N ticks
  - with after(ticks): / after(ticks, () => ...): Fires once after an elapsed tick delay
  - with when(cond): / when(cond, () => ...): Continuous trigger while condition holds
  - with until(cond): / until(cond, () => ...): Continues executing until condition becomes true
- Zero manual timer variable management
- Zero desynchronization risk (no fragile Wait() actions)
- Standalone compile via `eudkit build`
"""

from eudkit import (
    Add,
    CreateUnit,
    DisplayText,
    Ore,
    Player1,
    SetResources,
    SetTo,
    after,
    every,
    once,
    session,
    until,
    when,
)

kills = session.var(0, name="kills")


@session.fn
def game_loop():
    # 1. One-time initialization on game start
    with once():
        DisplayText("Welcome to StarCraft EUD!")
        SetResources(Player1, SetTo, 1000, Ore)

    # 2. Periodic passive income: every 24 ticks (1 second on Fastest speed)
    with every(24):
        SetResources(Player1, Add, 50, Ore)

    # 3. Timed reinforcement drop: arrives after 120 ticks (5 seconds)
    with after(120):
        DisplayText("Reinforcements have arrived!")
        CreateUnit(1, "Terran Marine", "Anywhere", Player1)

    # 4. Reactive milestone reward: triggers once kills reaches 10
    with when(kills >= 10):
        DisplayText("Combat Milestone: +500 Ore Reward")
        SetResources(Player1, Add, 500, Ore)
