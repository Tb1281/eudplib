"""Smoke-test fixture for the Zed extension.

Open this file in Zed, remove the text between the quotes on the `KillUnit`
line (or remove the closing quote too to reproduce a literal `KillUnit("`),
and request completion. The shared eudkit-lsp should offer all 228 canonical
unit names (for example `Terran Marine`).
"""

from eudplib import *


def kill_first_marine(player):
    KillUnit("Terran Marine", player)
