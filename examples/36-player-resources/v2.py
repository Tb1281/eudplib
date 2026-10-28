# 36 - player resources: periodic bonus / tax (modern eudkit)
#
# Expresses ideal intent:
# 1. Declarative periodic clock via `with every(84):` instead of manual frame counting.
# 2. Native `Accumulate` stock trigger conditions replace 16 expensive memory reads
#    and EUDFunc trampoline overhead.
# 3. Clean local session state with zero global singletons.
import eudkit


INTERVAL = 84          # ~5 s at Fastest (16.7 trigger loops / s)
LOW_MINERAL = 499      # <= 499 qualifies for relief
BONUS = 150
HIGH_GAS = 301         # >= 301 qualifies for tax
TAX = 50

from eudkit import session

paid_count = session.var(0)

@session.fn(main_loop=True)
def game_loop() -> None:
    with eudkit.every(INTERVAL):
        paid_count = 0
        for player in range(8):
            if eudkit.Accumulate(player, eudkit.AtMost, LOW_MINERAL, eudkit.Ore):
                eudkit.SetResources(player, eudkit.Add, BONUS, eudkit.Ore)
                paid_count += 1
            if eudkit.Accumulate(player, eudkit.AtLeast, HIGH_GAS, eudkit.Gas):
                eudkit.SetResources(player, eudkit.Subtract, TAX, eudkit.Gas)
        if paid_count >= 1:
            eudkit.DisplayText("Relief payments issued.")
