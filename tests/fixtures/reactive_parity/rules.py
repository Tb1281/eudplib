"""Reactive Rule fixture shared by the Python/TypeScript parity test."""

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


@session.fn(main_loop=True)
def main():
    with once():
        DisplayText("init")
    with once(kills >= 5):
        DisplayText("milestone")
    with when([kills >= 10, timer == 0]):
        DisplayText("standing")
    with until(timer == 0):
        DisplayText("countdown")
    with after(24):
        DisplayText("delayed")
    with every(12):
        DisplayText("periodic")
    SetResources(Player1, Add, 50, Ore)
