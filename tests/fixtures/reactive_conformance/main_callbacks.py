from eudkit import (
    Add,
    DisplayText,
    Ore,
    Player1,
    SetResources,
    after,
    every,
    once,
    session,
    until,
    when,
)

kills = session.var(0, name="kills")
timer = session.var(10, name="timer")
score = session.var(50, name="score")


@session.fn
def main():
    once(lambda: (DisplayText("once without condition"), SetResources(Player1, Add, 50, Ore)))
    once(score >= 100, lambda: DisplayText("once with condition"))
    when(kills >= 10, lambda: DisplayText("when condition"))
    until(timer == 0, lambda: DisplayText("until condition"))
    after(24, lambda: DisplayText("after delay"))
    every(12, lambda: DisplayText("every period"))
