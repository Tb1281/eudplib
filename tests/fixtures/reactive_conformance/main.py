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
    with once:
        DisplayText("once without condition")
        SetResources(Player1, Add, 50, Ore)

    with once(score >= 100):
        DisplayText("once with condition")

    with when(kills >= 10):
        DisplayText("when condition")

    with until(timer == 0):
        DisplayText("until condition")

    with after(24):
        DisplayText("after delay")

    with every(12):
        DisplayText("every period")
