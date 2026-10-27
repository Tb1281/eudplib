"""LEGACY function-call pattern: EUDFunc + EUDReturn (benchmark 47).

Same intent as ours.js: reusable game-time functions with
parameters and return values. calc_damage(atk, def) clamps damage
to >= 1 via an early EUDReturn; dist2(x1, y1, x2, y2) is a
squared-distance helper with sign-normalized deltas. run() calls
them for a hero attacking a target and drives a "critical" flag
from the results; a second call site uses literal arguments.

Legacy EUDFunc arguments are passed through pre-defined parameter
slots patched into the callee's own trigger fields; each call pays
the argument-copy + jump machinery, and the callee body is emitted
at its first call.
"""

from eudkit import *


@EUDFunc
def calc_damage(atk, defense):
    if EUDIf()(atk <= defense):
        EUDReturn(1)
    EUDEndIf()
    EUDReturn(atk - defense)


@EUDFunc
def dist2(x1, y1, x2, y2):
    dx = EUDVariable()
    dy = EUDVariable()
    dx << x1 - x2
    dy << y1 - y2
    if EUDIf()(dx < 0):
        dx << 0 - dx
    EUDEndIf()
    if EUDIf()(dy < 0):
        dy << 0 - dy
    EUDEndIf()
    EUDReturn(dx * dx + dy * dy)


hero_atk = EUDVariable()
hero_def = EUDVariable()
last_damage = EUDVariable(0)
last_dist2 = EUDVariable(0)
critical = EUDVariable(0)
hit_count = EUDVariable(0)
bonus_damage = EUDVariable(0)


def run():
    global hero_atk, hero_def, last_damage, last_dist2
    global critical, hit_count, bonus_damage
    hero_atk << 50
    hero_def << 20

    last_damage << calc_damage(hero_atk, hero_def)
    last_dist2 << dist2(100, 100, 160, 130)

    if EUDIf()(last_damage > 30):
        critical << 1
        hit_count += 1
    EUDEndIf()
    if EUDIf()(last_damage <= 30):
        critical << 0
    EUDEndIf()

    # A second call site with literal arguments (per-call machinery again).
    bonus_damage << calc_damage(80, 15)
    if EUDIf()(bonus_damage >= 60):
        hero_atk += 5
    EUDEndIf()

    # Range gate built from the distance helper's result.
    if EUDIf()(last_dist2 <= 10000):
        DoActions(DisplayText("In range"))
    EUDEndIf()
