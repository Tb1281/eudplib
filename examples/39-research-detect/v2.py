# 39 - research completion detection (modern eudkit)
#
# Expresses ideal intent:
# 1. Direct byte reads on research table via `bread(...)` with automatic EPD and byte masking.
# 2. Declarative `with once():` eliminates 8 manual boolean guard variables.
# 3. Clean local session state with zero global singletons.
import eudkit


RESEARCHED_BASE = 0x58CF44
STIM_PACKS = 0
LOCKDOWN = 1
BONUS_GAS = 100
LOCKDOWN_BONUS_ORE = 50

from eudkit import session

@session.fn(main_loop=True)
def game_loop() -> None:
    for player in range(4):
        stim_state = eudkit.bread(RESEARCHED_BASE + player * 24 + STIM_PACKS)
        if stim_state != 0:
            with eudkit.once():
                eudkit.SetResources(player, eudkit.Add, BONUS_GAS, eudkit.Gas)
                eudkit.DisplayText("Stim Packs research complete! +100 gas.")

        lockdown_state = eudkit.bread(RESEARCHED_BASE + player * 24 + LOCKDOWN)
        if lockdown_state != 0:
            with eudkit.once():
                eudkit.SetResources(player, eudkit.Add, LOCKDOWN_BONUS_ORE, eudkit.Ore)
                eudkit.DisplayText("Lockdown research complete! +50 ore.")
